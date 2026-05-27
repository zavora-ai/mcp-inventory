use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde_json::{json, Value};
use crate::types::*;
use crate::store::Store;

fn now() -> String { chrono::Utc::now().to_rfc3339() }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ItemInput { pub sku: String, pub name: String, pub category: String, pub unit: Option<String>, pub reorder_point: Option<f64>, pub reorder_qty: Option<f64>, pub cost: Option<f64>, pub currency: Option<String> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LocationInput { pub name: String, pub location_type: String, pub parent_id: Option<String>, pub address: Option<String>, /// Max units capacity
    pub capacity_units: Option<f64>, /// Max weight in kg
    pub capacity_weight_kg: Option<f64>, /// Max volume in m³
    pub capacity_volume_m3: Option<f64> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReceiveInput { pub sku: String, pub quantity: f64, pub location_id: String, pub reference: Option<String>, pub actor: String, pub lot_number: Option<String> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IssueInput { pub sku: String, pub quantity: f64, pub location_id: String, pub reference: Option<String>, pub actor: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TransferInput { pub sku: String, pub quantity: f64, pub from_location: String, pub to_location: String, pub actor: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AdjustInput { pub sku: String, pub location_id: String, pub new_quantity: f64, pub reason: String, pub actor: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StockQuery { pub sku: String, pub location_id: Option<String> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReserveInput { pub sku: String, pub location_id: String, pub quantity: f64, pub reference: String, pub expires_hours: Option<u32> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReserveIdInput { pub reservation_id: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BomInput { pub parent_sku: String, pub components: Vec<Value> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BomCheckInput { pub parent_sku: String, pub quantity: f64, pub location_id: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SkuInput { pub sku: String }

#[derive(Clone)]
pub struct InventoryServer { pub store: Store }
impl InventoryServer { pub fn new() -> Self { Self { store: Store::new() } } }

#[tool_router(server_handler)]
impl InventoryServer {
    #[tool(description = "Add or update an item in the inventory catalog (SKU, name, category, unit, reorder point).")]
    async fn item_upsert(&self, Parameters(input): Parameters<ItemInput>) -> String {
        let item = Item { sku: input.sku.clone(), name: input.name, category: input.category, unit: input.unit.unwrap_or_else(|| "each".into()), reorder_point: input.reorder_point.unwrap_or(10.0), reorder_qty: input.reorder_qty.unwrap_or(50.0), cost: input.cost.unwrap_or(0.0), currency: input.currency.unwrap_or_else(|| "USD".into()), attributes: json!({}) };
        self.store.items.lock().unwrap().insert(input.sku.clone(), item);
        json!({"status": "ok", "sku": input.sku}).to_string()
    }

    #[tool(description = "List all items in the inventory catalog.")]
    async fn item_list(&self) -> String {
        let items: Vec<_> = self.store.items.lock().unwrap().values().cloned().collect();
        json!({"count": items.len(), "items": items}).to_string()
    }

    #[tool(description = "Create a warehouse/zone/aisle/rack/bin location with optional capacity limits (units, weight, volume).")]
    async fn location_create(&self, Parameters(input): Parameters<LocationInput>) -> String {
        let id = Store::new_location_id();
        let loc = Location { id: id.clone(), name: input.name, location_type: input.location_type, parent_id: input.parent_id, address: input.address, capacity_units: input.capacity_units, capacity_weight_kg: input.capacity_weight_kg, capacity_volume_m3: input.capacity_volume_m3, used_units: 0.0, used_weight_kg: 0.0, used_volume_m3: 0.0 };
        self.store.locations.lock().unwrap().insert(id.clone(), loc);
        json!({"status": "created", "location_id": id}).to_string()
    }

    #[tool(description = "List all locations.")]
    async fn location_list(&self) -> String {
        let locs: Vec<_> = self.store.locations.lock().unwrap().values().cloned().collect();
        json!({"count": locs.len(), "locations": locs}).to_string()
    }

    #[tool(description = "Receive stock into a location (goods receipt from supplier, production, or return).")]
    async fn stock_receive(&self, Parameters(input): Parameters<ReceiveInput>) -> String {
        let m = StockMovement { id: Store::new_movement_id(), movement_type: "receive".into(), sku: input.sku.clone(), quantity: input.quantity, from_location: None, to_location: Some(input.location_id.clone()), reference: input.reference.unwrap_or_default(), actor: input.actor, lot_number: input.lot_number, timestamp: now() };
        self.store.record_movement(m);
        json!({"status": "received", "sku": input.sku, "quantity": input.quantity, "location": input.location_id}).to_string()
    }

    #[tool(description = "Issue stock from a location (sales, consumption, dispatch).")]
    async fn stock_issue(&self, Parameters(input): Parameters<IssueInput>) -> String {
        let avail = self.store.available_qty(&input.sku, &input.location_id);
        if avail < input.quantity { return json!({"error": "INSUFFICIENT_STOCK", "available": avail, "requested": input.quantity}).to_string(); }
        let m = StockMovement { id: Store::new_movement_id(), movement_type: "issue".into(), sku: input.sku.clone(), quantity: input.quantity, from_location: Some(input.location_id.clone()), to_location: None, reference: input.reference.unwrap_or_default(), actor: input.actor, lot_number: None, timestamp: now() };
        self.store.record_movement(m);
        json!({"status": "issued", "sku": input.sku, "quantity": input.quantity, "remaining": avail - input.quantity}).to_string()
    }

    #[tool(description = "Transfer stock between locations.")]
    async fn stock_transfer(&self, Parameters(input): Parameters<TransferInput>) -> String {
        let avail = self.store.available_qty(&input.sku, &input.from_location);
        if avail < input.quantity { return json!({"error": "INSUFFICIENT_STOCK", "available": avail}).to_string(); }
        let m = StockMovement { id: Store::new_movement_id(), movement_type: "transfer".into(), sku: input.sku.clone(), quantity: input.quantity, from_location: Some(input.from_location.clone()), to_location: Some(input.to_location.clone()), reference: String::new(), actor: input.actor, lot_number: None, timestamp: now() };
        self.store.record_movement(m);
        json!({"status": "transferred", "sku": input.sku, "quantity": input.quantity, "from": input.from_location, "to": input.to_location}).to_string()
    }

    #[tool(description = "Adjust stock quantity (cycle count correction, damage write-off, etc.).")]
    async fn stock_adjust(&self, Parameters(input): Parameters<AdjustInput>) -> String {
        let mut stock = self.store.stock.lock().unwrap();
        if let Some(s) = stock.iter_mut().find(|s| s.sku == input.sku && s.location_id == input.location_id) {
            let old = s.quantity;
            s.quantity = input.new_quantity;
            s.updated_at = now();
            json!({"status": "adjusted", "sku": input.sku, "old_qty": old, "new_qty": input.new_quantity, "reason": input.reason}).to_string()
        } else {
            stock.push(StockLevel { sku: input.sku.clone(), location_id: input.location_id.clone(), quantity: input.new_quantity, reserved: 0.0, lot_number: None, expiry_date: None, updated_at: now() });
            json!({"status": "created", "sku": input.sku, "quantity": input.new_quantity}).to_string()
        }
    }

    #[tool(description = "Check stock level for a SKU (optionally at a specific location). Shows available (total - reserved).")]
    async fn stock_check(&self, Parameters(input): Parameters<StockQuery>) -> String {
        let levels = self.store.get_stock(&input.sku, input.location_id.as_deref());
        let total_qty: f64 = levels.iter().map(|s| s.quantity).sum();
        let total_reserved: f64 = levels.iter().map(|s| s.reserved).sum();
        let item = self.store.items.lock().unwrap().get(&input.sku).cloned();
        let below_reorder = item.as_ref().map_or(false, |i| total_qty - total_reserved <= i.reorder_point);
        json!({"sku": input.sku, "total_quantity": total_qty, "reserved": total_reserved, "available": total_qty - total_reserved, "below_reorder_point": below_reorder, "locations": levels}).to_string()
    }

    #[tool(description = "Get items below reorder point (reorder alerts).")]
    async fn reorder_alerts(&self) -> String {
        let items = self.store.items.lock().unwrap().clone();
        let stock = self.store.stock.lock().unwrap().clone();
        let mut alerts = Vec::new();
        for item in items.values() {
            let total: f64 = stock.iter().filter(|s| s.sku == item.sku).map(|s| s.quantity - s.reserved).sum();
            if total <= item.reorder_point {
                alerts.push(json!({"sku": item.sku, "name": item.name, "available": total, "reorder_point": item.reorder_point, "suggested_order_qty": item.reorder_qty}));
            }
        }
        json!({"alerts": alerts.len(), "items": alerts}).to_string()
    }

    #[tool(description = "Reserve stock for an order (reduces available without reducing quantity). Prevents overselling.")]
    async fn stock_reserve(&self, Parameters(input): Parameters<ReserveInput>) -> String {
        let avail = self.store.available_qty(&input.sku, &input.location_id);
        if avail < input.quantity { return json!({"error": "INSUFFICIENT_STOCK", "available": avail}).to_string(); }
        let mut stock = self.store.stock.lock().unwrap();
        if let Some(s) = stock.iter_mut().find(|s| s.sku == input.sku && s.location_id == input.location_id) {
            s.reserved += input.quantity;
        }
        drop(stock);
        let expires = input.expires_hours.map(|h| (chrono::Utc::now() + chrono::Duration::hours(h as i64)).to_rfc3339());
        let id = Store::new_reservation_id();
        self.store.reservations.lock().unwrap().push(Reservation { id: id.clone(), sku: input.sku, location_id: input.location_id, quantity: input.quantity, reference: input.reference, expires_at: expires, created_at: now() });
        json!({"status": "reserved", "reservation_id": id}).to_string()
    }

    #[tool(description = "Release a stock reservation (cancel order, reservation expired).")]
    async fn stock_release(&self, Parameters(input): Parameters<ReserveIdInput>) -> String {
        let mut reservations = self.store.reservations.lock().unwrap();
        if let Some(idx) = reservations.iter().position(|r| r.id == input.reservation_id) {
            let res = reservations.remove(idx);
            let mut stock = self.store.stock.lock().unwrap();
            if let Some(s) = stock.iter_mut().find(|s| s.sku == res.sku && s.location_id == res.location_id) {
                s.reserved -= res.quantity;
            }
            json!({"status": "released", "sku": res.sku, "quantity": res.quantity}).to_string()
        } else {
            json!({"error": "RESERVATION_NOT_FOUND"}).to_string()
        }
    }

    #[tool(description = "Define a Bill of Materials (BOM) — components needed to build a parent item.")]
    async fn bom_set(&self, Parameters(input): Parameters<BomInput>) -> String {
        let mut bom = self.store.bom.lock().unwrap();
        bom.retain(|b| b.parent_sku != input.parent_sku);
        for c in &input.components {
            if let (Some(sku), Some(qty)) = (c["sku"].as_str(), c["quantity"].as_f64()) {
                bom.push(BomEntry { parent_sku: input.parent_sku.clone(), component_sku: sku.into(), quantity: qty });
            }
        }
        json!({"status": "ok", "parent_sku": input.parent_sku, "components": input.components.len()}).to_string()
    }

    #[tool(description = "Check BOM availability — can we build N units of a parent item with current stock?")]
    async fn bom_check(&self, Parameters(input): Parameters<BomCheckInput>) -> String {
        let bom: Vec<_> = self.store.bom.lock().unwrap().iter().filter(|b| b.parent_sku == input.parent_sku).cloned().collect();
        if bom.is_empty() { return json!({"error": "BOM_NOT_FOUND", "parent_sku": input.parent_sku}).to_string(); }
        let mut can_build = true;
        let mut shortages = Vec::new();
        for entry in &bom {
            let needed = entry.quantity * input.quantity;
            let avail = self.store.available_qty(&entry.component_sku, &input.location_id);
            if avail < needed {
                can_build = false;
                shortages.push(json!({"sku": entry.component_sku, "needed": needed, "available": avail, "shortage": needed - avail}));
            }
        }
        json!({"parent_sku": input.parent_sku, "quantity": input.quantity, "can_build": can_build, "shortages": shortages}).to_string()
    }

    #[tool(description = "Get stock movement history for a SKU.")]
    async fn movement_history(&self, Parameters(input): Parameters<SkuInput>) -> String {
        let movements: Vec<_> = self.store.movements.lock().unwrap().iter().filter(|m| m.sku == input.sku).cloned().collect();
        json!({"sku": input.sku, "count": movements.len(), "movements": movements}).to_string()
    }

    // === Pick/Pack/Ship ===

    #[tool(description = "Create a pick order for fulfillment. Allocates stock from locations and creates pick lines.")]
    async fn pick_create(&self, Parameters(input): Parameters<PickCreateInput>) -> String {
        let mut lines = Vec::new();
        for item in &input.items {
            let sku = item["sku"].as_str().unwrap_or_default();
            let qty = item["quantity"].as_f64().unwrap_or(1.0);
            let loc = item["location_id"].as_str().unwrap_or(&input.default_location);
            lines.push(PickLine { sku: sku.into(), quantity: qty, from_location: loc.into(), picked_qty: 0.0, status: "pending".into() });
        }
        let id = format!("pick_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let order = PickOrder { id: id.clone(), status: "pending".into(), order_reference: input.order_reference, lines, assigned_to: input.assigned_to, wave_id: None, created_at: now(), updated_at: now() };
        self.store.pick_orders.lock().unwrap().insert(id.clone(), order);
        json!({"status": "created", "pick_id": id}).to_string()
    }

    #[tool(description = "Confirm pick (mark items as picked). Moves status to 'picking' then 'packed'.")]
    async fn pick_confirm(&self, Parameters(input): Parameters<PickConfirmInput>) -> String {
        let mut picks = self.store.pick_orders.lock().unwrap();
        match picks.get_mut(&input.pick_id) {
            Some(p) => {
                for line in &mut p.lines {
                    if let Some(picked) = input.picked_skus.iter().find(|s| s["sku"].as_str() == Some(&line.sku)) {
                        line.picked_qty = picked["quantity"].as_f64().unwrap_or(line.quantity);
                        line.status = if line.picked_qty >= line.quantity { "picked".into() } else { "short".into() };
                    }
                }
                p.status = "packed".into();
                p.updated_at = now();
                json!({"status": "packed", "pick_id": input.pick_id}).to_string()
            }
            None => json!({"error": "PICK_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "Ship a pick order (mark as shipped, issues stock from locations).")]
    async fn pick_ship(&self, Parameters(input): Parameters<PickIdInput>) -> String {
        let mut picks = self.store.pick_orders.lock().unwrap();
        match picks.get_mut(&input.pick_id) {
            Some(p) => {
                p.status = "shipped".into();
                p.updated_at = now();
                // Issue stock for each picked line
                for line in &p.lines {
                    if line.picked_qty > 0.0 {
                        let mut stock = self.store.stock.lock().unwrap();
                        if let Some(s) = stock.iter_mut().find(|s| s.sku == line.sku && s.location_id == line.from_location) {
                            s.quantity -= line.picked_qty;
                            s.updated_at = now();
                        }
                    }
                }
                json!({"status": "shipped", "pick_id": input.pick_id}).to_string()
            }
            None => json!({"error": "PICK_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "List pick orders (optionally filter by status: pending, picking, packed, shipped).")]
    async fn pick_list(&self) -> String {
        let picks: Vec<_> = self.store.pick_orders.lock().unwrap().values().cloned().collect();
        json!({"count": picks.len(), "pick_orders": picks}).to_string()
    }

    // === Putaway ===

    #[tool(description = "Create a putaway rule (assign preferred location for items by category on receipt).")]
    async fn putaway_rule_create(&self, Parameters(input): Parameters<PutawayRuleInput>) -> String {
        let id = format!("put_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        self.store.putaway_rules.lock().unwrap().push(PutawayRule { id: id.clone(), category: input.category, target_zone: input.target_zone, priority: input.priority.unwrap_or(100) });
        json!({"status": "created", "rule_id": id}).to_string()
    }

    #[tool(description = "Suggest putaway location for an item based on category rules and available space.")]
    async fn putaway_suggest(&self, Parameters(input): Parameters<SkuInput>) -> String {
        let item = self.store.items.lock().unwrap().get(&input.sku).cloned();
        let category = item.map(|i| i.category).unwrap_or_default();
        let rules = self.store.putaway_rules.lock().unwrap().clone();
        let mut suggestions: Vec<_> = rules.iter().filter(|r| r.category == category || r.category == "*").collect();
        suggestions.sort_by_key(|r| r.priority);
        let locations = self.store.locations.lock().unwrap().clone();
        let suggested: Vec<_> = suggestions.iter().filter_map(|r| {
            locations.values().find(|l| l.id == r.target_zone || l.name == r.target_zone).map(|l| {
                let utilization = l.capacity_units.map(|cap| if cap > 0.0 { l.used_units / cap * 100.0 } else { 0.0 }).unwrap_or(0.0);
                json!({"location_id": l.id, "name": l.name, "type": l.location_type, "utilization_pct": utilization})
            })
        }).collect();
        json!({"sku": input.sku, "category": category, "suggestions": suggested}).to_string()
    }

    // === Cycle Counts ===

    #[tool(description = "Schedule a cycle count for a location.")]
    async fn cycle_count_schedule(&self, Parameters(input): Parameters<CycleCountInput>) -> String {
        let id = format!("cc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        self.store.cycle_counts.lock().unwrap().push(CycleCount { id: id.clone(), location_id: input.location_id, status: "scheduled".into(), scheduled_date: input.scheduled_date, counted_by: None, discrepancies: vec![], completed_at: None });
        json!({"status": "scheduled", "cycle_count_id": id}).to_string()
    }

    #[tool(description = "Complete a cycle count — submit actual counts and detect discrepancies.")]
    async fn cycle_count_complete(&self, Parameters(input): Parameters<CycleCountCompleteInput>) -> String {
        let mut counts = self.store.cycle_counts.lock().unwrap();
        if let Some(cc) = counts.iter_mut().find(|c| c.id == input.cycle_count_id) {
            cc.status = "completed".into();
            cc.counted_by = Some(input.counted_by);
            cc.completed_at = Some(now());
            // Check discrepancies
            let stock = self.store.stock.lock().unwrap();
            let mut discreps = Vec::new();
            for count in &input.counts {
                let sku = count["sku"].as_str().unwrap_or_default();
                let actual = count["actual_qty"].as_f64().unwrap_or(0.0);
                let system_qty: f64 = stock.iter().filter(|s| s.sku == sku && s.location_id == cc.location_id).map(|s| s.quantity).sum();
                if (actual - system_qty).abs() > 0.01 {
                    discreps.push(json!({"sku": sku, "system_qty": system_qty, "actual_qty": actual, "variance": actual - system_qty}));
                }
            }
            cc.discrepancies = discreps.clone();
            json!({"status": "completed", "cycle_count_id": input.cycle_count_id, "discrepancies": discreps.len(), "details": discreps}).to_string()
        } else {
            json!({"error": "CYCLE_COUNT_NOT_FOUND"}).to_string()
        }
    }

    // === Space Management ===

    #[tool(description = "Get space utilization for a location (or all locations). Shows capacity vs used for units, weight, and volume.")]
    async fn space_utilization(&self) -> String {
        let locations = self.store.locations.lock().unwrap().clone();
        let stock = self.store.stock.lock().unwrap().clone();
        let mut report: Vec<Value> = Vec::new();
        for loc in locations.values() {
            let total_units: f64 = stock.iter().filter(|s| s.location_id == loc.id).map(|s| s.quantity).sum();
            let unit_util = loc.capacity_units.map(|c| if c > 0.0 { total_units / c * 100.0 } else { 0.0 });
            report.push(json!({
                "location_id": loc.id, "name": loc.name, "type": loc.location_type,
                "units_used": total_units, "capacity_units": loc.capacity_units, "utilization_pct": unit_util,
                "capacity_weight_kg": loc.capacity_weight_kg, "capacity_volume_m3": loc.capacity_volume_m3
            }));
        }
        json!({"locations": report.len(), "report": report}).to_string()
    }
}

// --- Additional input types ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PickCreateInput {
    /// Order reference (e.g. "ORD-1234")
    pub order_reference: String,
    /// Items to pick: [{"sku": "...", "quantity": N, "location_id": "..."}]
    pub items: Vec<Value>,
    /// Default location if not specified per item
    pub default_location: String,
    /// Assign to picker
    pub assigned_to: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PickConfirmInput {
    /// Pick order ID
    pub pick_id: String,
    /// Picked items: [{"sku": "...", "quantity": N}]
    pub picked_skus: Vec<Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PickIdInput { pub pick_id: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PutawayRuleInput {
    /// Item category this rule applies to (or "*" for all)
    pub category: String,
    /// Target zone/location ID or name
    pub target_zone: String,
    /// Priority (lower = preferred)
    pub priority: Option<i32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CycleCountInput {
    /// Location to count
    pub location_id: String,
    /// Scheduled date (ISO)
    pub scheduled_date: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CycleCountCompleteInput {
    /// Cycle count ID
    pub cycle_count_id: String,
    /// Who performed the count
    pub counted_by: String,
    /// Actual counts: [{"sku": "...", "actual_qty": N}]
    pub counts: Vec<Value>,
}
