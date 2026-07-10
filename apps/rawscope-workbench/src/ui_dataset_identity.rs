//! Concise dataset identity for persistent workbench chrome.

use std::path::PathBuf;

use rawscope_data::{dataset_profile, DatasetIdentity, DatasetProfileId, DatasetSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasetDisplayIdentity {
    pub(crate) short_name: String,
    pub(crate) source_label: String,
    pub(crate) row_count: usize,
    pub(crate) profile_label: Option<String>,
    pub(crate) full_path: Option<PathBuf>,
}

impl DatasetDisplayIdentity {
    pub(crate) fn unavailable() -> Self {
        Self {
            short_name: "Dataset unavailable".to_string(),
            source_label: "Unavailable".to_string(),
            row_count: 0,
            profile_label: None,
            full_path: None,
        }
    }

    pub(crate) fn from_dataset(
        identity: &DatasetIdentity,
        profile_id: Option<DatasetProfileId>,
    ) -> Self {
        let (short_name, source_label, full_path) = match &identity.source {
            DatasetSource::Synthetic { generator, .. } => (
                format!("Synthetic {generator}"),
                "Synthetic".to_string(),
                None,
            ),
            DatasetSource::LocalCsv { path, .. } => (
                local_short_name(path),
                "CSV".to_string(),
                Some(path.clone()),
            ),
            DatasetSource::LocalParquet { path, .. } => (
                local_short_name(path),
                "Parquet".to_string(),
                Some(path.clone()),
            ),
        };

        Self {
            short_name,
            source_label,
            row_count: identity.row_count,
            profile_label: profile_id.map(|id| dataset_profile(id).display_name.to_string()),
            full_path,
        }
    }

    pub(crate) fn visible_label(&self) -> String {
        let base = format!("{} - {} rows", self.short_name, self.row_count_label());
        self.profile_label
            .as_ref()
            .map(|profile| format!("{base} - {profile}"))
            .unwrap_or(base)
    }

    pub(crate) fn row_count_label(&self) -> String {
        format_row_count(self.row_count)
    }

    pub(crate) fn window_title(&self) -> String {
        format!("RawScope - {}", self.short_name)
    }

    pub(crate) fn details_label(&self) -> String {
        let mut details = vec![format!("Source: {}", self.source_label)];
        if let Some(profile) = self.profile_label.as_ref() {
            details.push(format!("Profile: {profile}"));
        }
        if let Some(path) = self.full_path.as_ref() {
            details.push(format!("Path: {}", path.display()));
        }
        details.join("\n")
    }
}

fn local_short_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("local dataset")
        .to_string()
}

pub(crate) fn format_row_count(row_count: usize) -> String {
    let digits = row_count.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(character);
    }
    formatted
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rawscope_data::{DatasetIdentity, DatasetSource, VisualDatasetKind};

    use super::DatasetDisplayIdentity;

    #[test]
    fn local_dataset_identity_uses_filename_not_full_path() {
        let path = PathBuf::from(r"C:\datasets\lichess\games.csv");
        let identity = DatasetIdentity {
            visual_kind: VisualDatasetKind::Scatter,
            source: DatasetSource::LocalCsv {
                path: path.clone(),
                limit: None,
            },
            row_count: 343_000,
            field_bindings: Vec::new(),
            lane_labels: Vec::new(),
        };

        let display = DatasetDisplayIdentity::from_dataset(&identity, None);

        assert_eq!(display.visible_label(), "games.csv - 343,000 rows");
        assert!(!display.visible_label().contains("datasets"));
        assert_eq!(display.full_path.as_ref(), Some(&path));
    }

    #[test]
    fn synthetic_dataset_identity_has_no_path_action() {
        let identity = DatasetIdentity::synthetic_scatter(42, 20_000);

        let display = DatasetDisplayIdentity::from_dataset(&identity, None);

        assert_eq!(display.full_path, None);
    }

    #[test]
    fn window_title_uses_short_dataset_name() {
        let identity = DatasetIdentity {
            visual_kind: VisualDatasetKind::Scatter,
            source: DatasetSource::LocalParquet {
                path: PathBuf::from(r"C:\private\source\games.parquet"),
                limit: None,
            },
            row_count: 12,
            field_bindings: Vec::new(),
            lane_labels: Vec::new(),
        };

        let title = DatasetDisplayIdentity::from_dataset(&identity, None).window_title();

        assert_eq!(title, "RawScope - games.parquet");
        assert!(!title.contains("private"));
    }

    #[test]
    fn lichess_identity_hides_full_path_in_chrome() {
        let path = PathBuf::from(r"C:\private\lichess\games_profile.csv");
        let identity = DatasetIdentity {
            visual_kind: VisualDatasetKind::Scatter,
            source: DatasetSource::LocalCsv {
                path: path.clone(),
                limit: None,
            },
            row_count: 200_000,
            field_bindings: Vec::new(),
            lane_labels: Vec::new(),
        };
        let display = DatasetDisplayIdentity::from_dataset(
            &identity,
            Some(rawscope_data::DatasetProfileId::LichessGames),
        );

        assert_eq!(
            display.visible_label(),
            "games_profile.csv - 200,000 rows - Lichess games"
        );
        assert!(!display.visible_label().contains("private"));
        assert!(display
            .details_label()
            .contains(&path.display().to_string()));
    }
}
