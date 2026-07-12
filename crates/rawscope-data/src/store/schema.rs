//! Validated rectangular schema and stable column identity.

use std::{collections::HashMap, error::Error, fmt};

use rawscope_core::ColumnId;

/// The source/normalized type contract of one stored column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreColumnKind {
    Utf8,
    Bool,
    I64,
    U64,
    F64,
    TimestampMicros,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetColumn {
    id: ColumnId,
    name: String,
    kind: StoreColumnKind,
}

impl DatasetColumn {
    pub fn id(&self) -> ColumnId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> StoreColumnKind {
        self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetSchemaError {
    EmptyName { position: usize },
    DuplicateName { name: String },
    TooManyColumns { count: usize },
}

impl fmt::Display for DatasetSchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName { position } => {
                write!(formatter, "column {position} has an empty name")
            }
            Self::DuplicateName { name } => write!(formatter, "duplicate column name '{name}'"),
            Self::TooManyColumns { count } => write!(
                formatter,
                "schema has {count} columns, exceeding the u32 column-id space"
            ),
        }
    }
}

impl Error for DatasetSchemaError {}

/// An immutable schema with stable IDs assigned in validated order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetSchema {
    columns: Vec<DatasetColumn>,
    by_name: HashMap<String, ColumnId>,
}

impl DatasetSchema {
    pub fn try_new<I, N>(columns: I) -> Result<Self, DatasetSchemaError>
    where
        I: IntoIterator<Item = (N, StoreColumnKind)>,
        N: Into<String>,
    {
        let mut validated = Vec::new();
        let mut by_name = HashMap::new();
        for (position, (name, kind)) in columns.into_iter().enumerate() {
            let name = name.into();
            if name.trim().is_empty() {
                return Err(DatasetSchemaError::EmptyName { position });
            }
            if by_name.contains_key(&name) {
                return Err(DatasetSchemaError::DuplicateName { name });
            }
            let id = ColumnId::new(u32::try_from(position).map_err(|_| {
                DatasetSchemaError::TooManyColumns {
                    count: position + 1,
                }
            })?);
            by_name.insert(name.clone(), id);
            validated.push(DatasetColumn { id, name, kind });
        }
        Ok(Self {
            columns: validated,
            by_name,
        })
    }

    pub fn columns(&self) -> &[DatasetColumn] {
        &self.columns
    }

    pub fn column(&self, id: ColumnId) -> Option<&DatasetColumn> {
        self.columns.get(id.get() as usize)
    }

    pub fn column_id(&self, name: &str) -> Option<ColumnId> {
        self.by_name.get(name).copied()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}
