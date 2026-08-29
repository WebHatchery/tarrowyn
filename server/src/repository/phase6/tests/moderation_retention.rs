use super::super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::ModerationReportResponse;

fn report(index: usize) -> ModerationReportResponse {
    ModerationReportResponse {
        request_id: format!("report-request-{index}"),
        accepted: true,
        report_id: format!("report-{index}"),
        status: "queued".to_owned(),
        reason: None,
    }
}

#[test]
fn moderation_reports_expire_and_keep_a_bounded_recent_window() {
    let repository = WorldRepository::new(ServerConfig::default());
    let now = 10_000_000;
    let mut state = repository.state.lock().expect("repository lock");
    state.phase6.reports = (0..(super::super::MAX_MODERATION_REPORTS + 1))
        .map(|index| {
            let response = report(index);
            (response.report_id.clone(), response)
        })
        .collect();
    state.phase6.report_created_at = (0..(super::super::MAX_MODERATION_REPORTS + 1))
        .map(|index| (format!("report-{index}"), now - 513 + index as u64))
        .collect();
    super::super::trim_moderation_reports(&mut state.phase6, now);

    assert_eq!(
        state.phase6.reports.len(),
        super::super::MAX_MODERATION_REPORTS
    );
    assert!(!state.phase6.reports.contains_key("report-0"));
    assert!(state.phase6.reports.contains_key("report-512"));

    state
        .phase6
        .reports
        .insert("report-old".to_owned(), report(900));
    state.phase6.report_created_at.insert(
        "report-old".to_owned(),
        now - super::super::MODERATION_REPORT_RETENTION_SECONDS,
    );
    super::super::trim_moderation_reports(&mut state.phase6, now);
    assert!(!state.phase6.reports.contains_key("report-old"));
}
