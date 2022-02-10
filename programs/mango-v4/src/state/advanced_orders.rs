

pub struct AnyAdvancedOrder {
    pub advanced_order_type: AdvancedOrderType,
    pub is_active: bool,
    pub padding: [u8; ADVANCED_ORDER_SIZE - 2],
}

pub struct PerpTriggerOrder {
    pub advanced_order_type: AdvancedOrderType,
    pub is_active: bool,
    pub market_index: u8,
    pub order_type: OrderType,
    pub side: Side,
    pub trigger_condition: TriggerCondition, // Bid & Below => Take profit on short, Bid & Above => stop loss on short
    pub reduce_only: bool,                   // only valid on perp order
    pub padding0: [u8; 1],
    pub client_order_id: u64,
    pub price: i64,
    pub quantity: i64,
    pub trigger_price: I80F48,

    /// Padding for expansion
    pub padding1: [u8; 32],
}

pub struct AdvancedOrders {
    pub meta_data: MetaData,
    pub orders: [AnyAdvancedOrder; MAX_ADVANCED_ORDERS],
}