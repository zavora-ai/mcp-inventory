use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde_json::{json, Value};
use reqwest::Client;
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
pub struct InventoryServer {
    pub store: Store,
    pub client: Client,
    pub grocy_url: Option<String>,
    pub grocy_key: Option<String>,
    pub shopify_store: Option<String>,
    pub shopify_token: Option<String>,
    pub pancake_key: Option<String>,
    pub pancake_shop: Option<String>,
}
impl InventoryServer {
    pub fn new() -> Self {
        Self {
            store: Store::new(),
            client: Client::builder().build().unwrap_or_default(),
            grocy_url: std::env::var("GROCY_URL").ok(),
            grocy_key: std::env::var("GROCY_API_KEY").ok(),
            shopify_store: std::env::var("SHOPIFY_STORE").ok(),
            shopify_token: std::env::var("SHOPIFY_ACCESS_TOKEN").ok(),
            pancake_key: std::env::var("PANCAKE_POS_API_KEY").ok(),
            pancake_shop: std::env::var("PANCAKE_POS_SHOP_ID").ok(),
        }
    }
}

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

    // === Wave Planning ===

    #[tool(description = "Create a wave (batch multiple pick orders together for efficient warehouse picking).")]
    async fn wave_create(&self, Parameters(input): Parameters<WaveCreateInput>) -> String {
        let id = format!("wave_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let wave = Wave { id: id.clone(), name: input.name, status: "planning".into(), pick_ids: input.pick_ids.clone(), priority: input.priority.unwrap_or_else(|| "medium".into()), created_at: now(), released_at: None, completed_at: None };
        // Link picks to wave
        let mut picks = self.store.pick_orders.lock().unwrap();
        for pid in &input.pick_ids {
            if let Some(p) = picks.get_mut(pid) { p.wave_id = Some(id.clone()); }
        }
        drop(picks);
        self.store.waves.lock().unwrap().insert(id.clone(), wave);
        json!({"status": "created", "wave_id": id, "pick_orders": input.pick_ids.len()}).to_string()
    }

    #[tool(description = "Release a wave (moves all pick orders in the wave to 'picking' status, assigns to pickers).")]
    async fn wave_release(&self, Parameters(input): Parameters<WaveIdInput>) -> String {
        let mut waves = self.store.waves.lock().unwrap();
        match waves.get_mut(&input.wave_id) {
            Some(w) => {
                w.status = "in_progress".into();
                w.released_at = Some(now());
                let pick_ids = w.pick_ids.clone();
                drop(waves);
                let mut picks = self.store.pick_orders.lock().unwrap();
                for pid in &pick_ids {
                    if let Some(p) = picks.get_mut(pid) { p.status = "picking".into(); }
                }
                json!({"status": "released", "wave_id": input.wave_id, "picks_released": pick_ids.len()}).to_string()
            }
            None => json!({"error": "WAVE_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "Complete a wave (marks wave as completed when all picks are shipped).")]
    async fn wave_complete(&self, Parameters(input): Parameters<WaveIdInput>) -> String {
        let mut waves = self.store.waves.lock().unwrap();
        match waves.get_mut(&input.wave_id) {
            Some(w) => {
                w.status = "completed".into();
                w.completed_at = Some(now());
                json!({"status": "completed", "wave_id": input.wave_id}).to_string()
            }
            None => json!({"error": "WAVE_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "List waves with their status and pick order counts.")]
    async fn wave_list(&self) -> String {
        let waves: Vec<_> = self.store.waves.lock().unwrap().values().cloned().collect();
        json!({"count": waves.len(), "waves": waves}).to_string()
    }

    // === Barcode / Label Generation ===

    #[tool(description = "Generate a barcode label for a SKU, location, lot, shipment, or receipt. Returns barcode value and label text for printing.")]
    async fn label_generate(&self, Parameters(input): Parameters<LabelInput>) -> String {
        let barcode_value = match input.barcode_type.as_str() {
            "sku" => format!("SKU-{}", input.entity_id),
            "location" => format!("LOC-{}", input.entity_id),
            "lot" => format!("LOT-{}", input.entity_id),
            "shipment" => format!("SHP-{}", input.entity_id),
            "receipt" => format!("RCV-{}", input.entity_id),
            _ => format!("ID-{}", input.entity_id),
        };
        let format = input.barcode_format.unwrap_or_else(|| "code128".into());
        let mut label_text = vec![barcode_value.clone()];
        // Add context info
        if input.barcode_type == "sku" {
            if let Some(item) = self.store.items.lock().unwrap().get(&input.entity_id) {
                label_text.push(item.name.clone());
                label_text.push(format!("Cat: {}", item.category));
            }
        } else if input.barcode_type == "location" {
            if let Some(loc) = self.store.locations.lock().unwrap().get(&input.entity_id) {
                label_text.push(loc.name.clone());
                label_text.push(format!("Type: {}", loc.location_type));
            }
        }
        if let Some(ref extra) = input.extra_text { label_text.extend(extra.clone()); }

        let id = format!("lbl_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let label = BarcodeLabel { id: id.clone(), barcode_type: input.barcode_type, entity_id: input.entity_id, barcode_format: format.clone(), barcode_value: barcode_value.clone(), label_text: label_text.clone(), generated_at: now() };
        self.store.labels.lock().unwrap().push(label);
        json!({"label_id": id, "barcode_value": barcode_value, "barcode_format": format, "label_text": label_text, "printable": true}).to_string()
    }

    #[tool(description = "Generate labels in batch (multiple SKUs, locations, or shipments at once).")]
    async fn label_batch(&self, Parameters(input): Parameters<LabelBatchInput>) -> String {
        let format = input.barcode_format.unwrap_or_else(|| "code128".into());
        let mut labels = Vec::new();
        for entity_id in &input.entity_ids {
            let barcode_value = match input.barcode_type.as_str() {
                "sku" => format!("SKU-{}", entity_id),
                "location" => format!("LOC-{}", entity_id),
                "lot" => format!("LOT-{}", entity_id),
                _ => format!("ID-{}", entity_id),
            };
            let id = format!("lbl_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
            let label = BarcodeLabel { id: id.clone(), barcode_type: input.barcode_type.clone(), entity_id: entity_id.clone(), barcode_format: format.clone(), barcode_value: barcode_value.clone(), label_text: vec![barcode_value.clone()], generated_at: now() };
            labels.push(json!({"label_id": id, "entity_id": entity_id, "barcode_value": barcode_value}));
            self.store.labels.lock().unwrap().push(label);
        }
        json!({"count": labels.len(), "barcode_format": format, "labels": labels}).to_string()
    }

    // === Serialized Warehousing ===

    #[tool(description = "Register a serialized item (individual unit tracking by serial number). Optionally link RFID tag.")]
    async fn serial_register(&self, Parameters(input): Parameters<SerialRegisterInput>) -> String {
        let qr = format!("QR:SN={}&SKU={}&LOC={}", input.serial_number, input.sku, input.location_id);
        let item = SerializedItem {
            serial_number: input.serial_number.clone(), sku: input.sku, status: "in_stock".into(),
            location_id: input.location_id.clone(), lot_number: input.lot_number,
            manufacture_date: input.manufacture_date, expiry_date: input.expiry_date,
            rfid_tag: input.rfid_tag.clone(), qr_code: Some(qr.clone()),
            history: vec![SerialEvent { event_type: "received".into(), location: input.location_id, actor: "system".into(), timestamp: now(), reference: None }],
            metadata: json!({}),
        };
        self.store.serialized.lock().unwrap().insert(input.serial_number.clone(), item);
        // If RFID tag provided, register it too
        if let Some(epc) = input.rfid_tag {
            self.store.rfid_tags.lock().unwrap().insert(epc.clone(), RfidTag { epc, serial_number: Some(input.serial_number.clone()), sku: None, location_id: String::new(), last_read_at: now(), read_count: 0, status: "active".into() });
        }
        json!({"status": "registered", "serial_number": input.serial_number, "qr_code": qr}).to_string()
    }

    #[tool(description = "Move a serialized item to a new location (tracks full chain of custody).")]
    async fn serial_move(&self, Parameters(input): Parameters<SerialMoveInput>) -> String {
        let mut items = self.store.serialized.lock().unwrap();
        match items.get_mut(&input.serial_number) {
            Some(item) => {
                let event_type = input.event_type.unwrap_or_else(|| "moved".into());
                item.location_id = input.to_location.clone();
                item.status = match event_type.as_str() { "shipped" => "shipped", "scrapped" => "scrapped", "returned" => "in_stock", _ => "in_stock" }.into();
                item.history.push(SerialEvent { event_type: event_type.clone(), location: input.to_location, actor: input.actor, timestamp: now(), reference: input.reference });
                json!({"status": "moved", "serial_number": input.serial_number, "event": event_type, "history_length": item.history.len()}).to_string()
            }
            None => json!({"error": "SERIAL_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "Look up a serialized item by serial number (full history, location, status).")]
    async fn serial_lookup(&self, Parameters(input): Parameters<SerialQueryInput>) -> String {
        match self.store.serialized.lock().unwrap().get(&input.serial_number) {
            Some(item) => serde_json::to_string_pretty(item).unwrap_or_default(),
            None => json!({"error": "SERIAL_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "List all serialized items at a location.")]
    async fn serial_scan_location(&self, Parameters(input): Parameters<SerialScanInput>) -> String {
        let items: Vec<_> = self.store.serialized.lock().unwrap().values().filter(|i| i.location_id == input.location_id).cloned().collect();
        json!({"location_id": input.location_id, "count": items.len(), "items": items.iter().map(|i| json!({"serial": i.serial_number, "sku": i.sku, "status": i.status, "rfid": i.rfid_tag, "lot": i.lot_number})).collect::<Vec<_>>()}).to_string()
    }

    // === RFID ===

    #[tool(description = "Register an RFID tag (EPC) and link it to a serial number or SKU.")]
    async fn rfid_register(&self, Parameters(input): Parameters<RfidRegisterInput>) -> String {
        let tag = RfidTag { epc: input.epc.clone(), serial_number: input.serial_number, sku: input.sku, location_id: input.location_id, last_read_at: now(), read_count: 0, status: "active".into() };
        self.store.rfid_tags.lock().unwrap().insert(input.epc.clone(), tag);
        json!({"status": "registered", "epc": input.epc}).to_string()
    }

    #[tool(description = "Process RFID reader scan — bulk update tag locations and detect missing/unexpected tags.")]
    async fn rfid_bulk_read(&self, Parameters(input): Parameters<RfidReadInput>) -> String {
        let mut tags = self.store.rfid_tags.lock().unwrap();
        let mut found = Vec::new();
        let mut unknown = Vec::new();
        for epc in &input.epcs {
            if let Some(tag) = tags.get_mut(epc) {
                tag.location_id = input.location_id.clone();
                tag.last_read_at = now();
                tag.read_count += 1;
                found.push(json!({"epc": epc, "serial": tag.serial_number, "sku": tag.sku}));
            } else {
                unknown.push(epc.clone());
            }
        }
        // Detect missing — tags expected at this location but not scanned
        let missing: Vec<_> = tags.values().filter(|t| t.location_id == input.location_id && t.status == "active" && !input.epcs.contains(&t.epc)).map(|t| json!({"epc": t.epc, "serial": t.serial_number})).collect();
        json!({"location_id": input.location_id, "scanned": input.epcs.len(), "found": found.len(), "unknown": unknown.len(), "missing": missing.len(), "found_tags": found, "unknown_epcs": unknown, "missing_tags": missing}).to_string()
    }

    #[tool(description = "Look up an RFID tag by EPC.")]
    async fn rfid_lookup(&self, Parameters(input): Parameters<RfidQueryInput>) -> String {
        match self.store.rfid_tags.lock().unwrap().get(&input.epc) {
            Some(tag) => serde_json::to_string_pretty(tag).unwrap_or_default(),
            None => json!({"error": "RFID_NOT_FOUND", "epc": input.epc}).to_string(),
        }
    }

    // === QR Code ===

    #[tool(description = "Generate a QR code payload for a serial, SKU, location, or shipment. Returns the encoded data string.")]
    async fn qr_generate(&self, Parameters(input): Parameters<QrGenerateInput>) -> String {
        let base = match input.entity_type.as_str() {
            "serial" => {
                let item = self.store.serialized.lock().unwrap().get(&input.entity_id).cloned();
                match item {
                    Some(i) => format!("SN={}&SKU={}&LOT={}&LOC={}", i.serial_number, i.sku, i.lot_number.unwrap_or_default(), i.location_id),
                    None => format!("SN={}", input.entity_id),
                }
            }
            "sku" => format!("SKU={}", input.entity_id),
            "location" => format!("LOC={}", input.entity_id),
            "shipment" => format!("SHP={}", input.entity_id),
            _ => format!("ID={}", input.entity_id),
        };
        let payload = if let Some(extra) = input.extra_data { format!("{}&DATA={}", base, extra) } else { base };
        let label_id = format!("lbl_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        self.store.labels.lock().unwrap().push(BarcodeLabel { id: label_id.clone(), barcode_type: input.entity_type.clone(), entity_id: input.entity_id, barcode_format: "qr".into(), barcode_value: payload.clone(), label_text: vec![payload.clone()], generated_at: now() });
        json!({"label_id": label_id, "format": "qr", "payload": payload, "printable": true}).to_string()
    }

    // === External Backend Sync ===

    #[tool(description = "Sync inventory with Grocy (self-hosted grocery/inventory manager). Pull imports Grocy stock into local store. Push exports local stock to Grocy. Requires GROCY_URL, GROCY_API_KEY env vars.")]
    async fn sync_grocy(&self, Parameters(input): Parameters<GrocySyncInput>) -> String {
        let (Some(url), Some(key)) = (&self.grocy_url, &self.grocy_key) else {
            return json!({"error": "GROCY_NOT_CONFIGURED", "message": "Set GROCY_URL and GROCY_API_KEY"}).to_string();
        };
        match input.direction.as_str() {
            "pull" => {
                let endpoint = format!("{}/api/stock", url);
                match self.client.get(&endpoint).header("GROCY-API-KEY", key.as_str()).send().await {
                    Ok(resp) => match resp.json::<Vec<Value>>().await {
                        Ok(items) => {
                            let mut synced = 0;
                            for item in &items {
                                let sku = item["product"]["name"].as_str().unwrap_or_default();
                                if input.sku.as_ref().map_or(true, |s| s == sku) {
                                    let qty = item["amount"].as_f64().unwrap_or(0.0);
                                    self.store.items.lock().unwrap().entry(sku.into()).or_insert_with(|| Item { sku: sku.into(), name: sku.into(), category: "grocy".into(), unit: "each".into(), reorder_point: item["product"]["min_stock_amount"].as_f64().unwrap_or(0.0), reorder_qty: 10.0, cost: 0.0, currency: "USD".into(), attributes: json!({}) });
                                    let mut stock = self.store.stock.lock().unwrap();
                                    if let Some(s) = stock.iter_mut().find(|s| s.sku == sku && s.location_id == "grocy") {
                                        s.quantity = qty; s.updated_at = now();
                                    } else {
                                        stock.push(StockLevel { sku: sku.into(), location_id: "grocy".into(), quantity: qty, reserved: 0.0, lot_number: None, expiry_date: item["best_before_date"].as_str().map(String::from), updated_at: now() });
                                    }
                                    synced += 1;
                                }
                            }
                            json!({"status": "pulled", "source": "grocy", "items_synced": synced}).to_string()
                        }
                        Err(e) => json!({"error": e.to_string()}).to_string(),
                    },
                    Err(e) => json!({"error": e.to_string()}).to_string(),
                }
            }
            "push" => {
                json!({"status": "push_not_yet_implemented", "message": "Grocy push requires product ID mapping. Use pull to import first."}).to_string()
            }
            _ => json!({"error": "Invalid direction. Use 'pull' or 'push'"}).to_string(),
        }
    }

    #[tool(description = "Sync inventory with Shopify. Pull imports Shopify inventory levels. Push updates Shopify stock from local. Requires SHOPIFY_STORE, SHOPIFY_ACCESS_TOKEN env vars.")]
    async fn sync_shopify(&self, Parameters(input): Parameters<ShopifySyncInput>) -> String {
        let (Some(store), Some(token)) = (&self.shopify_store, &self.shopify_token) else {
            return json!({"error": "SHOPIFY_NOT_CONFIGURED", "message": "Set SHOPIFY_STORE and SHOPIFY_ACCESS_TOKEN"}).to_string();
        };
        let base = format!("https://{}.myshopify.com/admin/api/2024-01", store);
        match input.direction.as_str() {
            "pull" => {
                let url = format!("{}/inventory_levels.json?limit=50", base);
                match self.client.get(&url).header("X-Shopify-Access-Token", token.as_str()).send().await {
                    Ok(resp) => match resp.json::<Value>().await {
                        Ok(data) => {
                            let levels = data["inventory_levels"].as_array().unwrap_or(&vec![]).clone();
                            let mut synced = 0;
                            for level in &levels {
                                let item_id = level["inventory_item_id"].to_string();
                                let qty = level["available"].as_f64().unwrap_or(0.0);
                                let loc = level["location_id"].to_string();
                                let mut stock = self.store.stock.lock().unwrap();
                                if let Some(s) = stock.iter_mut().find(|s| s.sku == item_id && s.location_id == format!("shopify_{}", loc)) {
                                    s.quantity = qty; s.updated_at = now();
                                } else {
                                    stock.push(StockLevel { sku: item_id, location_id: format!("shopify_{}", loc), quantity: qty, reserved: 0.0, lot_number: None, expiry_date: None, updated_at: now() });
                                }
                                synced += 1;
                            }
                            json!({"status": "pulled", "source": "shopify", "levels_synced": synced}).to_string()
                        }
                        Err(e) => json!({"error": e.to_string()}).to_string(),
                    },
                    Err(e) => json!({"error": e.to_string()}).to_string(),
                }
            }
            "push" => {
                let Some(ref sku) = input.sku else {
                    return json!({"error": "SKU required for push"}).to_string();
                };
                let loc_id = input.location_id.as_deref().unwrap_or("default");
                let qty: f64 = self.store.stock.lock().unwrap().iter().filter(|s| s.sku == *sku).map(|s| s.quantity - s.reserved).sum();
                let url = format!("{}/inventory_levels/set.json", base);
                let body = json!({"location_id": loc_id, "inventory_item_id": sku, "available": qty as i64});
                match self.client.post(&url).header("X-Shopify-Access-Token", token.as_str()).json(&body).send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        json!({"status": if status < 400 { "pushed" } else { "failed" }, "sku": sku, "quantity": qty, "http_status": status}).to_string()
                    }
                    Err(e) => json!({"error": e.to_string()}).to_string(),
                }
            }
            _ => json!({"error": "Invalid direction. Use 'pull' or 'push'"}).to_string(),
        }
    }

    #[tool(description = "Sync inventory with Pancake POS. Pull imports warehouse/inventory data. Push exports stock levels. Requires PANCAKE_POS_API_KEY, PANCAKE_POS_SHOP_ID env vars.")]
    async fn sync_pancake(&self, Parameters(input): Parameters<PancakeSyncInput>) -> String {
        let (Some(key), Some(shop)) = (&self.pancake_key, &self.pancake_shop) else {
            return json!({"error": "PANCAKE_NOT_CONFIGURED", "message": "Set PANCAKE_POS_API_KEY and PANCAKE_POS_SHOP_ID"}).to_string();
        };
        let base = format!("https://pos.pages.fm/api/v1/shops/{}", shop);
        match input.direction.as_str() {
            "pull" => {
                let url = format!("{}/inventory?warehouse_id={}", base, input.warehouse_id.as_deref().unwrap_or("all"));
                match self.client.get(&url).header("Authorization", format!("Bearer {}", key)).send().await {
                    Ok(resp) => match resp.json::<Value>().await {
                        Ok(data) => {
                            let items = data["data"].as_array().unwrap_or(&vec![]).clone();
                            let mut synced = 0;
                            for item in &items {
                                let sku = item["product_id"].to_string();
                                let qty = item["quantity"].as_f64().unwrap_or(0.0);
                                let wh = item["warehouse_id"].to_string();
                                let mut stock = self.store.stock.lock().unwrap();
                                if let Some(s) = stock.iter_mut().find(|s| s.sku == sku && s.location_id == format!("pancake_{}", wh)) {
                                    s.quantity = qty; s.updated_at = now();
                                } else {
                                    stock.push(StockLevel { sku, location_id: format!("pancake_{}", wh), quantity: qty, reserved: 0.0, lot_number: None, expiry_date: None, updated_at: now() });
                                }
                                synced += 1;
                            }
                            json!({"status": "pulled", "source": "pancake_pos", "items_synced": synced}).to_string()
                        }
                        Err(e) => json!({"error": e.to_string()}).to_string(),
                    },
                    Err(e) => json!({"error": e.to_string()}).to_string(),
                }
            }
            "push" => {
                json!({"status": "push_planned", "message": "Pancake POS push requires product mapping. Use pull first to establish SKU links."}).to_string()
            }
            _ => json!({"error": "Invalid direction. Use 'pull' or 'push'"}).to_string(),
        }
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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WaveCreateInput {
    /// Wave name (e.g. "Morning Wave", "Priority Rush")
    pub name: String,
    /// Pick order IDs to include in this wave
    pub pick_ids: Vec<String>,
    /// Priority: low, medium, high, critical
    pub priority: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WaveIdInput { pub wave_id: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LabelInput {
    /// Type: sku, location, lot, shipment, receipt
    pub barcode_type: String,
    /// Entity ID (SKU code, location ID, lot number, etc.)
    pub entity_id: String,
    /// Barcode format: code128, ean13, qr, datamatrix (default: code128)
    pub barcode_format: Option<String>,
    /// Extra text lines to print on label
    pub extra_text: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LabelBatchInput {
    /// Type: sku, location, lot, shipment
    pub barcode_type: String,
    /// List of entity IDs to generate labels for
    pub entity_ids: Vec<String>,
    /// Barcode format (default: code128)
    pub barcode_format: Option<String>,
}

// === Serialized / RFID / QR Input Types ===

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SerialRegisterInput {
    /// Serial number (unique identifier for this individual unit)
    pub serial_number: String,
    /// SKU this serial belongs to
    pub sku: String,
    /// Location where item is stored
    pub location_id: String,
    /// Lot/batch number
    pub lot_number: Option<String>,
    /// Manufacture date
    pub manufacture_date: Option<String>,
    /// Expiry date
    pub expiry_date: Option<String>,
    /// RFID EPC tag (if tagged)
    pub rfid_tag: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SerialMoveInput {
    /// Serial number
    pub serial_number: String,
    /// New location
    pub to_location: String,
    /// Actor
    pub actor: String,
    /// Event type: moved, picked, shipped, returned, scrapped
    pub event_type: Option<String>,
    /// Reference (order ID, etc.)
    pub reference: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SerialQueryInput {
    /// Serial number to look up
    pub serial_number: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SerialScanInput {
    /// Location ID to scan
    pub location_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RfidRegisterInput {
    /// EPC (Electronic Product Code, 96-bit hex string)
    pub epc: String,
    /// Serial number to link (optional)
    pub serial_number: Option<String>,
    /// SKU to link (optional)
    pub sku: Option<String>,
    /// Location where tag is
    pub location_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RfidReadInput {
    /// Location ID where RFID reader is scanning
    pub location_id: String,
    /// EPCs detected by the reader
    pub epcs: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RfidQueryInput {
    /// EPC to look up
    pub epc: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QrGenerateInput {
    /// Entity type: serial, sku, location, shipment
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Extra data to encode in QR (JSON string)
    pub extra_data: Option<Value>,
}

// === Sync Backend Input Types ===

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GrocySyncInput {
    /// Direction: pull (Grocy→local) or push (local→Grocy)
    pub direction: String,
    /// SKU filter (optional, sync all if omitted)
    pub sku: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShopifySyncInput {
    /// Direction: pull (Shopify→local) or push (local→Shopify)
    pub direction: String,
    /// SKU filter (optional)
    pub sku: Option<String>,
    /// Shopify location ID (required for push)
    pub location_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PancakeSyncInput {
    /// Direction: pull (Pancake→local) or push (local→Pancake)
    pub direction: String,
    /// Warehouse ID filter (optional)
    pub warehouse_id: Option<String>,
}
