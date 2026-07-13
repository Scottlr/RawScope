//! Generic session field-role adaptation into the T014 controller contract.

use std::io;

use rawscope_analysis::visual_field::{VisualFieldMapping, VisualFieldProjection};
use rawscope_data::{DatasetSchema, LoadedColumnKind, LoadedSourceTable};
use rawscope_session::ResolvedSessionView;

/// Validates a persisted category binding against the loaded source before a
/// renderer or controller resource is created.
pub(crate) fn validate_category_binding(
    source: &LoadedSourceTable,
    category: Option<&str>,
) -> Result<(), io::Error> {
    let Some(category) = category else {
        return Ok(());
    };
    let column = source
        .columns
        .iter()
        .find(|column| column.name == category)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("session category binding '{category}' is not present in the source"),
            )
        })?;
    if !matches!(
        column.kind,
        LoadedColumnKind::Integer | LoadedColumnKind::String
    ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "session category binding '{category}' has incompatible source kind {}",
                column.kind
            ),
        ));
    }
    Ok(())
}

/// Resolves session role names once against the typed schema and constructs the
/// same checked mapping consumed by the T014 visual-field controller.
pub(crate) fn resolve_visual_field_mapping(
    view: &ResolvedSessionView,
    schema: &DatasetSchema,
) -> Result<VisualFieldMapping, io::Error> {
    let column_id = |name: &str| {
        schema.column_id(name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("session visual binding '{name}' is not present in the typed schema"),
            )
        })
    };
    let (projection, category) = match view {
        ResolvedSessionView::NumericPair { x, y, category, .. } => (
            VisualFieldProjection::NumericPair {
                x: column_id(x)?,
                y: column_id(y)?,
            },
            category.as_deref().map(column_id).transpose()?,
        ),
        ResolvedSessionView::TimeValue {
            time,
            value,
            category,
            ..
        } => (
            VisualFieldProjection::TimeValue {
                time: column_id(time)?,
                value: column_id(value)?,
            },
            category.as_deref().map(column_id).transpose()?,
        ),
        ResolvedSessionView::TimelineLane { .. } => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "timeline lane bindings do not form a two-dimensional visual field",
            ));
        }
    };
    VisualFieldMapping::try_new(schema, projection, category).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("session visual field mapping is incompatible: {error:?}"),
        )
    })
}
