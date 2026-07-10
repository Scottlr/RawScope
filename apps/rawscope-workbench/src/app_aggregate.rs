//! Workbench-owned aggregate overview cache rebuilding.

use rawscope_render::{
    scatter_aggregate_overview, timeline_aggregate_overview, AggregateCacheConfig,
};
use tracing::error;

use crate::app::WorkbenchApp;

pub(crate) const DEFAULT_AGGREGATE_OVERVIEW_WIDTH: u32 = 96;
pub(crate) const DEFAULT_AGGREGATE_OVERVIEW_HEIGHT: u32 = 64;
pub(crate) const DEFAULT_AGGREGATE_ROW_SAMPLE_COUNT: usize = 8;

impl WorkbenchApp {
    pub(crate) fn clear_aggregate_overviews(&mut self) {
        self.scatter.scatter_aggregate_overview = None;
        self.timeline.timeline_aggregate_overview = None;
    }

    pub(crate) fn rebuild_scatter_aggregate_overview(&mut self) {
        let Some(viewport) = self.scatter.viewport else {
            self.scatter.scatter_aggregate_overview = None;
            return;
        };

        let config = aggregate_cache_config();
        self.scatter.scatter_aggregate_overview = match scatter_aggregate_overview(
            &self.scatter.points,
            viewport.full_x_range(),
            viewport.full_y_range(),
            config,
        ) {
            Ok(overview) => Some(overview),
            Err(err) => {
                error!(
                    error = %err,
                    grid_width = DEFAULT_AGGREGATE_OVERVIEW_WIDTH,
                    grid_height = DEFAULT_AGGREGATE_OVERVIEW_HEIGHT,
                    "failed to rebuild scatter aggregate overview"
                );
                None
            }
        };
    }

    pub(crate) fn rebuild_timeline_aggregate_overview(&mut self) {
        let Some(viewport) = self.timeline.viewport else {
            self.timeline.timeline_aggregate_overview = None;
            return;
        };

        let config = aggregate_cache_config();
        self.timeline.timeline_aggregate_overview = match timeline_aggregate_overview(
            &self.timeline.events,
            viewport.full_time_range(),
            viewport.lane_count(),
            config,
        ) {
            Ok(overview) => Some(overview),
            Err(err) => {
                error!(
                    error = %err,
                    grid_width = DEFAULT_AGGREGATE_OVERVIEW_WIDTH,
                    grid_height = DEFAULT_AGGREGATE_OVERVIEW_HEIGHT,
                    lane_count = viewport.lane_count(),
                    "failed to rebuild timeline aggregate overview"
                );
                None
            }
        };
    }
}

fn aggregate_cache_config() -> AggregateCacheConfig {
    AggregateCacheConfig {
        grid_width: DEFAULT_AGGREGATE_OVERVIEW_WIDTH,
        grid_height: DEFAULT_AGGREGATE_OVERVIEW_HEIGHT,
        max_row_ids_per_bin: DEFAULT_AGGREGATE_ROW_SAMPLE_COUNT,
    }
}

#[cfg(test)]
mod tests {
    use rawscope_core::{F32Range, RowId, U64Range};
    use rawscope_data::{
        ScatterPointKind, ScatterPointRecord, TimelineEventKind, TimelineEventRecord,
    };
    use rawscope_render::{
        AggregateBinSample, ScatterAggregateOverview, TimelineAggregateOverview,
    };

    use super::*;
    use crate::{app::WorkbenchApp, demo::DemoMode};

    #[test]
    fn clear_aggregate_overviews_removes_stale_dataset_caches() {
        let mut app = WorkbenchApp::default();
        app.scatter.scatter_aggregate_overview = Some(ScatterAggregateOverview {
            x_range: F32Range::new(0.0, 1.0),
            y_range: F32Range::new(0.0, 1.0),
            grid_width: 1,
            grid_height: 1,
            max_bin_count: 1,
            bins: vec![AggregateBinSample {
                count: 1,
                row_ids: vec![RowId(1)],
            }],
        });
        app.timeline.timeline_aggregate_overview = Some(TimelineAggregateOverview {
            time_range: U64Range::new(0, 1),
            lane_count: 1,
            grid_width: 1,
            grid_height: 1,
            max_bin_count: 1,
            bins: vec![AggregateBinSample {
                count: 1,
                row_ids: vec![RowId(2)],
            }],
        });

        app.clear_aggregate_overviews();

        assert!(app.scatter.scatter_aggregate_overview.is_none());
        assert!(app.timeline.timeline_aggregate_overview.is_none());
    }

    #[test]
    fn rebuild_scatter_aggregate_overview_uses_full_view_ranges() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Scatter;
        app.scatter.viewport = Some(rawscope_render::ScatterViewport::new(
            F32Range::new(0.0, 8.0),
            F32Range::new(0.0, 8.0),
        ));
        app.scatter.points = vec![
            ScatterPointRecord {
                row_id: RowId(1),
                x: 1.0,
                y: 1.0,
                kind: ScatterPointKind::Unclassified,
            },
            ScatterPointRecord {
                row_id: RowId(2),
                x: 1.05,
                y: 1.05,
                kind: ScatterPointKind::Unclassified,
            },
        ];

        app.rebuild_scatter_aggregate_overview();

        let overview = app
            .scatter
            .scatter_aggregate_overview
            .as_ref()
            .expect("scatter aggregate overview should exist");
        assert_eq!(overview.x_range, F32Range::new(0.0, 8.0));
        assert_eq!(overview.y_range, F32Range::new(0.0, 8.0));
        assert_eq!(
            overview.bins.len(),
            (DEFAULT_AGGREGATE_OVERVIEW_WIDTH as usize)
                * (DEFAULT_AGGREGATE_OVERVIEW_HEIGHT as usize)
        );
        assert_eq!(overview.max_bin_count, 2);

        app.scatter.viewport = None;
        app.rebuild_scatter_aggregate_overview();

        assert!(app.scatter.scatter_aggregate_overview.is_none());
    }

    #[test]
    fn rebuild_timeline_aggregate_overview_uses_full_view_ranges() {
        let mut app = WorkbenchApp::default();
        app.demo_mode = DemoMode::Timeline;
        app.timeline.viewport = Some(rawscope_render::TimelineViewport::new(
            U64Range::new(100, 500),
            3,
        ));
        app.timeline.events = vec![
            TimelineEventRecord {
                row_id: RowId(4),
                timestamp: 499,
                lane: 2,
                value: 1.0,
                kind: TimelineEventKind::Unclassified,
            },
            TimelineEventRecord {
                row_id: RowId(5),
                timestamp: 500,
                lane: 2,
                value: 2.0,
                kind: TimelineEventKind::Unclassified,
            },
        ];

        app.rebuild_timeline_aggregate_overview();

        let overview = app
            .timeline
            .timeline_aggregate_overview
            .as_ref()
            .expect("timeline aggregate overview should exist");
        assert_eq!(overview.time_range, U64Range::new(100, 500));
        assert_eq!(overview.lane_count, 3);
        assert_eq!(
            overview.bins.len(),
            (DEFAULT_AGGREGATE_OVERVIEW_WIDTH as usize)
                * (DEFAULT_AGGREGATE_OVERVIEW_HEIGHT as usize)
        );
        assert_eq!(overview.max_bin_count, 2);

        app.timeline.viewport = None;
        app.rebuild_timeline_aggregate_overview();

        assert!(app.timeline.timeline_aggregate_overview.is_none());
    }
}
