//! Bounded preview/exact render coordination before final controller migration.

use std::collections::BTreeMap;

pub(crate) use rawscope_render::VisualFieldIntent;

pub(crate) const MAX_LIVE_GPU_TICKETS: usize = 4;
pub(crate) const MAX_PREVIEW_GPU_TICKETS: usize = MAX_LIVE_GPU_TICKETS - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RenderTicketId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewportIntent {
    pub(crate) dataset_generation: u64,
    pub(crate) cohort_generation: u64,
    pub(crate) viewport_generation: u64,
    pub(crate) visual_field: Option<VisualFieldIntent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactSettleIntent {
    pub(crate) viewport: ViewportIntent,
    pub(crate) request_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderTicketKind {
    Preview,
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LiveTicket {
    kind: RenderTicketKind,
    viewport: ViewportIntent,
    request_generation: u64,
    publishable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettledRenderGeneration {
    pub(crate) viewport: ViewportIntent,
    pub(crate) request_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmitDecision {
    Accepted(RenderTicketId),
    Coalesced,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Completion {
    Published,
    Retired,
    IgnoredStale,
    Unknown,
}

#[derive(Debug, Default)]
pub(crate) struct RenderCoordinator {
    next_ticket_id: u64,
    live: BTreeMap<RenderTicketId, LiveTicket>,
    latest_preview: Option<ViewportIntent>,
    latest_exact: Option<ExactSettleIntent>,
    active: Option<SettledRenderGeneration>,
}

impl RenderCoordinator {
    pub(crate) fn request_preview(&mut self, viewport: ViewportIntent) -> SubmitDecision {
        if self.preview_count() >= MAX_PREVIEW_GPU_TICKETS {
            self.latest_preview = Some(viewport);
            return SubmitDecision::Coalesced;
        }
        SubmitDecision::Accepted(self.accept(RenderTicketKind::Preview, viewport, 0))
    }

    pub(crate) fn request_exact(&mut self, intent: ExactSettleIntent) -> SubmitDecision {
        if self.exact_count() > 0 || self.live.len() >= MAX_LIVE_GPU_TICKETS {
            self.latest_exact = Some(intent);
            return SubmitDecision::Deferred;
        }
        SubmitDecision::Accepted(self.accept(
            RenderTicketKind::Exact,
            intent.viewport,
            intent.request_generation,
        ))
    }

    pub(crate) fn take_latest_preview(&mut self) -> Option<ViewportIntent> {
        self.latest_preview.take()
    }

    pub(crate) fn take_latest_exact(&mut self) -> Option<ExactSettleIntent> {
        self.latest_exact.take()
    }

    pub(crate) fn mark_obsolete(&mut self, ticket_id: RenderTicketId) -> bool {
        let Some(ticket) = self.live.get_mut(&ticket_id) else {
            return false;
        };
        ticket.publishable = false;
        true
    }

    pub(crate) fn complete(
        &mut self,
        ticket_id: RenderTicketId,
        result: Result<SettledRenderGeneration, ()>,
    ) -> Completion {
        let Some(ticket) = self.live.remove(&ticket_id) else {
            return Completion::Unknown;
        };
        if ticket.kind != RenderTicketKind::Exact || !ticket.publishable {
            return Completion::Retired;
        }
        let Ok(generation) = result else {
            return Completion::Retired;
        };
        if generation.viewport != ticket.viewport
            || generation.request_generation != ticket.request_generation
        {
            return Completion::IgnoredStale;
        }
        self.active = Some(generation);
        Completion::Published
    }

    pub(crate) fn active(&self) -> Option<SettledRenderGeneration> {
        self.active
    }

    pub(crate) fn live_count(&self) -> usize {
        self.live.len()
    }

    pub(crate) fn preview_count(&self) -> usize {
        self.live
            .values()
            .filter(|ticket| ticket.kind == RenderTicketKind::Preview)
            .count()
    }

    fn exact_count(&self) -> usize {
        self.live
            .values()
            .filter(|ticket| ticket.kind == RenderTicketKind::Exact)
            .count()
    }

    fn accept(
        &mut self,
        kind: RenderTicketKind,
        viewport: ViewportIntent,
        request_generation: u64,
    ) -> RenderTicketId {
        self.next_ticket_id = self.next_ticket_id.saturating_add(1);
        let ticket_id = RenderTicketId(self.next_ticket_id);
        self.live.insert(
            ticket_id,
            LiveTicket {
                kind,
                viewport,
                request_generation,
                publishable: true,
            },
        );
        ticket_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawscope_analysis::visual_field::{
        VisualFieldMapping, VisualFieldMode, VisualFieldProjection,
    };
    use rawscope_core::{ColumnId, GridSize};
    use rawscope_data::{DatasetGenerationCounter, DatasetSchema, StoreColumnKind};
    use rawscope_render::{ResolutionDecisionReason, VisualResolutionDecision};

    fn viewport(generation: u64) -> ViewportIntent {
        ViewportIntent {
            dataset_generation: 1,
            cohort_generation: 2,
            viewport_generation: generation,
            visual_field: None,
        }
    }

    fn visual_intent(mode: VisualFieldMode) -> VisualFieldIntent {
        let schema =
            DatasetSchema::try_new([("x", StoreColumnKind::F64), ("y", StoreColumnKind::F64)])
                .unwrap();
        let mapping = VisualFieldMapping::try_new(
            &schema,
            VisualFieldProjection::NumericPair {
                x: ColumnId::new(0),
                y: ColumnId::new(1),
            },
            None,
        )
        .unwrap();
        let mut datasets = DatasetGenerationCounter::default();
        let mut cohorts = rawscope_analysis::cohort::CohortGenerationCounter::default();
        VisualFieldIntent {
            dataset_generation: datasets.mint(),
            cohort_generation: cohorts.mint(),
            view_generation: rawscope_render::VisualFieldViewGenerationCounter::default().mint(),
            mapping,
            mode,
            resolution: VisualResolutionDecision {
                grid: GridSize::new(32, 16),
                quality: rawscope_render::VisualFieldQuality::Exact,
                reason: ResolutionDecisionReason::PlotMatched,
            },
        }
    }

    #[test]
    fn preview_capacity_coalesces_without_growing_registry() {
        let mut coordinator = RenderCoordinator::default();
        for generation in 0..MAX_PREVIEW_GPU_TICKETS as u64 {
            assert!(matches!(
                coordinator.request_preview(viewport(generation)),
                SubmitDecision::Accepted(_)
            ));
        }
        assert_eq!(coordinator.live_count(), MAX_PREVIEW_GPU_TICKETS);
        assert_eq!(
            coordinator.request_preview(viewport(99)),
            SubmitDecision::Coalesced
        );
        assert_eq!(coordinator.live_count(), MAX_PREVIEW_GPU_TICKETS);
        assert_eq!(coordinator.take_latest_preview(), Some(viewport(99)));
    }

    #[test]
    fn exact_work_is_deferred_until_reserved_capacity_is_free() {
        let mut coordinator = RenderCoordinator::default();
        for generation in 0..MAX_PREVIEW_GPU_TICKETS as u64 {
            let _ = coordinator.request_preview(viewport(generation));
        }
        let intent = ExactSettleIntent {
            viewport: viewport(10),
            request_generation: 7,
        };
        assert!(matches!(
            coordinator.request_exact(intent),
            SubmitDecision::Accepted(_)
        ));
        assert_eq!(coordinator.live_count(), MAX_LIVE_GPU_TICKETS);
        assert_eq!(
            coordinator.request_exact(ExactSettleIntent {
                request_generation: 8,
                ..intent
            }),
            SubmitDecision::Deferred
        );
        assert_eq!(
            coordinator.take_latest_exact().unwrap().request_generation,
            8
        );
    }

    #[test]
    fn stale_or_failed_exact_completion_preserves_active_generation() {
        let mut coordinator = RenderCoordinator::default();
        let first = ExactSettleIntent {
            viewport: viewport(1),
            request_generation: 1,
        };
        let first_id = match coordinator.request_exact(first) {
            SubmitDecision::Accepted(ticket_id) => ticket_id,
            _ => panic!("exact request should be accepted"),
        };
        assert_eq!(
            coordinator.complete(
                first_id,
                Ok(SettledRenderGeneration {
                    viewport: first.viewport,
                    request_generation: first.request_generation,
                })
            ),
            Completion::Published
        );

        let second = ExactSettleIntent {
            viewport: viewport(2),
            request_generation: 2,
        };
        let second_id = match coordinator.request_exact(second) {
            SubmitDecision::Accepted(ticket_id) => ticket_id,
            _ => panic!("second exact request should be accepted"),
        };
        assert_eq!(
            coordinator.complete(second_id, Err(())),
            Completion::Retired
        );
        assert_eq!(coordinator.active().unwrap().request_generation, 1);
    }

    #[test]
    fn obsolete_ticket_retires_exactly_once() {
        let mut coordinator = RenderCoordinator::default();
        let intent = ExactSettleIntent {
            viewport: viewport(1),
            request_generation: 1,
        };
        let ticket_id = match coordinator.request_exact(intent) {
            SubmitDecision::Accepted(ticket_id) => ticket_id,
            _ => panic!("exact request should be accepted"),
        };
        assert!(coordinator.mark_obsolete(ticket_id));
        assert_eq!(
            coordinator.complete(
                ticket_id,
                Ok(SettledRenderGeneration {
                    viewport: intent.viewport,
                    request_generation: intent.request_generation,
                })
            ),
            Completion::Retired
        );
        assert_eq!(
            coordinator.complete(ticket_id, Err(())),
            Completion::Unknown
        );
    }

    #[test]
    fn stale_mode_completion_cannot_publish() {
        let mut coordinator = RenderCoordinator::default();
        let mut requested_viewport = viewport(1);
        requested_viewport.visual_field = Some(visual_intent(VisualFieldMode::Density));
        let ticket_id = match coordinator.request_exact(ExactSettleIntent {
            viewport: requested_viewport,
            request_generation: 1,
        }) {
            SubmitDecision::Accepted(ticket_id) => ticket_id,
            _ => panic!("exact request should be accepted"),
        };

        let mut stale_viewport = requested_viewport;
        stale_viewport.visual_field = Some(visual_intent(VisualFieldMode::CohortComparison));
        assert_eq!(
            coordinator.complete(
                ticket_id,
                Ok(SettledRenderGeneration {
                    viewport: stale_viewport,
                    request_generation: 1,
                }),
            ),
            Completion::IgnoredStale
        );
        assert_eq!(coordinator.active(), None);
    }

    #[test]
    fn pointer_intents_coalesce_without_resource_growth() {
        let mut coordinator = RenderCoordinator::default();
        for generation in 0..MAX_PREVIEW_GPU_TICKETS as u64 {
            let mut request = viewport(generation);
            request.visual_field = Some(visual_intent(VisualFieldMode::Density));
            assert!(matches!(
                coordinator.request_preview(request),
                SubmitDecision::Accepted(_)
            ));
        }
        let mut latest = viewport(99);
        latest.visual_field = Some(visual_intent(VisualFieldMode::DensityRidges));
        assert_eq!(
            coordinator.request_preview(latest),
            SubmitDecision::Coalesced
        );
        assert_eq!(coordinator.live_count(), MAX_PREVIEW_GPU_TICKETS);
        assert_eq!(coordinator.take_latest_preview(), Some(latest));
    }
}
