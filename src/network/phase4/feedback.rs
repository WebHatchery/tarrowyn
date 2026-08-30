use tarrowyn_protocol::{
    ClaimLifecycleStatus, ClaimRecord, GovernanceAction, GovernanceRequest, GovernanceResponse,
    GovernanceState, ProfessionAction, ProfessionKind, ProfessionRequest, ProposalStatus,
    ServiceOrder, ServiceOrderStatus,
};

pub(super) fn claim_success_message(claim: Option<&ClaimRecord>) -> String {
    let Some(claim) = claim else {
        return "The land registry recorded the lease lifecycle.".to_owned();
    };
    let plot = format!("({}, {})", claim.position.x, claim.position.y);
    match claim.status {
        ClaimLifecycleStatus::Requested => {
            format!("Lease requested for plot {plot}; approval remains with the Town hall.")
        }
        ClaimLifecycleStatus::Active => format!(
            "Lease active at plot {plot}; building access is open for {} days.",
            claim.lease_days
        ),
        ClaimLifecycleStatus::Renewed => format!(
            "Lease renewed at plot {plot}; another {} days are recorded.",
            claim.lease_days
        ),
        ClaimLifecycleStatus::Transferred => format!(
            "Lease transferred to {}; plot {plot} keeps its recognised history.",
            claim.owner_name.as_deref().unwrap_or("the receiving resident")
        ),
        ClaimLifecycleStatus::Inherited => format!(
            "Lease inherited by {}; plot {plot} keeps its recognised history.",
            claim.owner_name.as_deref().unwrap_or("the receiving resident")
        ),
        ClaimLifecycleStatus::Abandoned => format!(
            "Lease abandoned at plot {plot}; use the Registry control to reclaim it after the grace period."
        ),
        ClaimLifecycleStatus::Expired => format!(
            "Lease expired at plot {plot}; the registry is holding it through the reclamation grace period."
        ),
        ClaimLifecycleStatus::Reclaimed => format!(
            "Lease reclaimed; plot {plot} is back in the available land ledger."
        ),
    }
}

pub(super) fn profession_success_message(
    order: Option<&ServiceOrder>,
    request: Option<&ProfessionRequest>,
) -> String {
    let Some(order) = order else {
        if let Some(request) = request {
            return match request.action {
                ProfessionAction::LearnCapability => request
                    .profession
                    .map(|profession| {
                        format!(
                            "{} capability recorded; its credential is now in the profession ledger.",
                            profession_name(profession)
                        )
                    })
                    .unwrap_or_else(|| {
                        "Professional capability recorded in the profession ledger.".to_owned()
                    }),
                ProfessionAction::Inspect => {
                    "Profession ledger inspected; materials and service orders are current."
                        .to_owned()
                }
                _ => "The profession ledger recorded the requested action.".to_owned(),
            };
        }
        return "The profession ledger recorded the requested action.".to_owned();
    };
    match order.status {
        ServiceOrderStatus::Open => format!(
            "Service order posted: {}; {} gold reward is on the board.",
            order.service, order.reward_gold
        ),
        ServiceOrderStatus::Accepted => format!(
            "Service order accepted: {}; {} is responsible for the {} gold reward.",
            order.service,
            order.provider_name.as_deref().unwrap_or("a named provider"),
            order.reward_gold
        ),
        ServiceOrderStatus::Completed => {
            let benefit = order.benefit.trim_end_matches('.');
            if benefit.is_empty() {
                format!(
                    "Service order completed: {} at {}% quality; {} gold paid.",
                    order.service, order.quality, order.reward_gold
                )
            } else {
                format!(
                    "Service order completed: {} at {}% quality; {} gold paid. {benefit}.",
                    order.service, order.quality, order.reward_gold
                )
            }
        }
        ServiceOrderStatus::Cancelled => {
            format!("Service order cancelled: {}.", order.service)
        }
    }
}

fn profession_name(profession: ProfessionKind) -> &'static str {
    match profession {
        ProfessionKind::Farmer => "Farmer",
        ProfessionKind::Smith => "Smith",
        ProfessionKind::Carpenter => "Carpenter",
        ProfessionKind::Healer => "Healer",
        ProfessionKind::Scout => "Scout",
        ProfessionKind::Steward => "Steward",
    }
}

pub(super) fn governance_success_message(
    response: &GovernanceResponse,
    request: Option<&GovernanceRequest>,
) -> String {
    let Some(request) = request else {
        return "The town-hall ledger recorded the public action.".to_owned();
    };
    match request.action {
        GovernanceAction::ClaimOffice => {
            let office = request.office_id.as_deref().and_then(|office_id| {
                response
                    .governance
                    .offices
                    .iter()
                    .find(|office| office.office_id == office_id)
            });
            office
                .map(|office| {
                    format!(
                        "Town hall recorded the {} office; {} now holds it.",
                        office.title,
                        office.holder_name.as_deref().unwrap_or("a named resident")
                    )
                })
                .unwrap_or_else(|| "The town-hall ledger recorded the public action.".to_owned())
        }
        GovernanceAction::SetTaxRate => {
            let rate = response
                .governance
                .taxation
                .as_ref()
                .map(|policy| policy.rate_percent)
                .unwrap_or(0);
            format!(
                "Public settlement tax is now {rate}%; the public treasury holds {} gold.",
                response.governance.public_treasury
            )
        }
        GovernanceAction::Propose => response
            .governance
            .proposals
            .last()
            .map(|proposal| {
                format!(
                    "Public proposal posted: {} for {}; {} public gold awaits Town hall approval.",
                    proposal.action.label(),
                    proposal.target,
                    proposal.cost
                )
            })
            .unwrap_or_else(|| "The town-hall ledger recorded the public action.".to_owned()),
        GovernanceAction::Approve => request
            .proposal_id
            .as_deref()
            .and_then(|proposal_id| {
                response
                    .governance
                    .proposals
                    .iter()
                    .find(|proposal| proposal.proposal_id == proposal_id)
            })
            .filter(|proposal| proposal.status == tarrowyn_protocol::ProposalStatus::Approved)
            .map(|proposal| {
                format!(
                    "Public proposal approved: {}; use the Town hall control to complete it.",
                    proposal.target
                )
            })
            .unwrap_or_else(|| "The town-hall ledger recorded the public action.".to_owned()),
        GovernanceAction::Complete => request
            .proposal_id
            .as_deref()
            .and_then(|proposal_id| {
                response
                    .governance
                    .proposals
                    .iter()
                    .find(|proposal| proposal.proposal_id == proposal_id)
            })
            .filter(|proposal| proposal.status == tarrowyn_protocol::ProposalStatus::Completed)
            .map(|proposal| {
                format!(
                    "Public action completed: {} for {}; {} public gold spent.",
                    proposal.action.label(),
                    proposal.target,
                    proposal.cost
                )
            })
            .unwrap_or_else(|| "The town-hall ledger recorded the public action.".to_owned()),
        GovernanceAction::Inspect => governance_inspection_message(&response.governance),
    }
}

fn governance_inspection_message(governance: &GovernanceState) -> String {
    let filled_offices = governance
        .offices
        .iter()
        .filter(|office| !office.vacant)
        .count();
    let open_proposals = governance
        .proposals
        .iter()
        .filter(|proposal| {
            matches!(
                proposal.status,
                ProposalStatus::Proposed | ProposalStatus::Approved
            )
        })
        .count();
    let tax_rate = governance
        .taxation
        .as_ref()
        .map(|policy| policy.rate_percent)
        .unwrap_or(0);
    format!(
        "Town hall ledger: {filled_offices}/{} offices filled • {open_proposals} proposals in progress • treasury {} gold • tax {tax_rate}% • administration {}%.",
        governance.offices.len(),
        governance.public_treasury,
        governance.administration_quality,
    )
}
