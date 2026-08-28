use super::*;

#[test]
fn maintenance_status_prefers_the_deployment_message_and_has_a_safe_fallback() {
    assert_eq!(
        maintenance_status_message(true, Some("The road opens after dawn.")),
        Some("Maintenance: The road opens after dawn.".to_owned())
    );
    assert_eq!(
        maintenance_status_message(false, None),
        Some("The settlement is in maintenance; tap Reconnect when it is ready.".to_owned())
    );
    assert_eq!(maintenance_status_message(true, None), None);
}
