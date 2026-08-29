use super::*;
use tarrowyn_protocol::MarketOrderAction;

impl Phase5Client {
    pub(super) fn queue_market(&mut self, request_id: String) {
        let Some(region) = self.region.as_ref() else {
            return;
        };
        let location = region.player_location_id.as_str();
        if let Some(order) = self.market.as_ref().and_then(|market| {
            market.orders.iter().find(|order| {
                order.status == tarrowyn_protocol::MarketOrderStatus::Open
                    && order.destination_location_id == location
            })
        }) {
            super::super::queue::try_push(
                &mut self.commands,
                Phase5Command::Market(MarketOrderRequest {
                    request_id,
                    action: MarketOrderAction::Fulfil,
                    order_id: Some(order.order_id.clone()),
                    destination_location_id: None,
                    commodity: None,
                    quantity: None,
                }),
            );
        } else if location == "hearth"
            && !self.market.as_ref().is_some_and(|market| {
                market.orders.iter().any(|order| {
                    order.status == tarrowyn_protocol::MarketOrderStatus::Open
                        && self
                            .own_account_id
                            .as_deref()
                            .is_some_and(|account_id| order.owner_account_id == account_id)
                })
            })
        {
            super::super::queue::try_push(
                &mut self.commands,
                Phase5Command::Market(MarketOrderRequest {
                    request_id,
                    action: MarketOrderAction::Create,
                    order_id: None,
                    destination_location_id: Some("saltmere".to_owned()),
                    commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                    quantity: Some(1),
                }),
            );
        }
    }

    pub(super) fn queue_market_cancel(&mut self, request_id: String) {
        let own = self.own_account_id.as_deref();
        let order_id = self.market.as_ref().and_then(|market| {
            market
                .orders
                .iter()
                .find(|order| {
                    order.status == tarrowyn_protocol::MarketOrderStatus::Open
                        && own.is_some_and(|account_id| order.owner_account_id == account_id)
                })
                .map(|order| order.order_id.clone())
        });
        let Some(order_id) = order_id else {
            return;
        };
        super::super::queue::try_push(
            &mut self.commands,
            Phase5Command::Market(MarketOrderRequest {
                request_id,
                action: MarketOrderAction::Cancel,
                order_id: Some(order_id),
                destination_location_id: None,
                commodity: None,
                quantity: None,
            }),
        );
    }
}
