use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::*;

fn now() -> String { chrono::Utc::now().to_rfc3339() }
fn uid() -> String { uuid::Uuid::new_v4().to_string()[..8].to_string() }

#[derive(Clone)]
pub struct Store {
    pub items: Arc<Mutex<HashMap<String, Item>>>,
    pub locations: Arc<Mutex<HashMap<String, Location>>>,
    pub stock: Arc<Mutex<Vec<StockLevel>>>,
    pub movements: Arc<Mutex<Vec<StockMovement>>>,
    pub bom: Arc<Mutex<Vec<BomEntry>>>,
    pub reservations: Arc<Mutex<Vec<Reservation>>>,
    pub pick_orders: Arc<Mutex<HashMap<String, PickOrder>>>,
    pub putaway_rules: Arc<Mutex<Vec<PutawayRule>>>,
    pub cycle_counts: Arc<Mutex<Vec<CycleCount>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            locations: Arc::new(Mutex::new(HashMap::new())),
            stock: Arc::new(Mutex::new(Vec::new())),
            movements: Arc::new(Mutex::new(Vec::new())),
            bom: Arc::new(Mutex::new(Vec::new())),
            reservations: Arc::new(Mutex::new(Vec::new())),
            pick_orders: Arc::new(Mutex::new(HashMap::new())),
            putaway_rules: Arc::new(Mutex::new(Vec::new())),
            cycle_counts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_stock(&self, sku: &str, location_id: Option<&str>) -> Vec<StockLevel> {
        self.stock.lock().unwrap().iter().filter(|s| {
            s.sku == sku && location_id.map_or(true, |l| s.location_id == l)
        }).cloned().collect()
    }

    pub fn available_qty(&self, sku: &str, location_id: &str) -> f64 {
        self.stock.lock().unwrap().iter()
            .filter(|s| s.sku == sku && s.location_id == location_id)
            .map(|s| s.quantity - s.reserved)
            .sum()
    }

    pub fn record_movement(&self, m: StockMovement) {
        // Update stock levels
        let mut stock = self.stock.lock().unwrap();
        if let Some(ref from) = m.from_location {
            if let Some(s) = stock.iter_mut().find(|s| s.sku == m.sku && s.location_id == *from) {
                s.quantity -= m.quantity;
                s.updated_at = now();
            }
        }
        if let Some(ref to) = m.to_location {
            if let Some(s) = stock.iter_mut().find(|s| s.sku == m.sku && s.location_id == *to) {
                s.quantity += m.quantity;
                s.updated_at = now();
            } else {
                stock.push(StockLevel { sku: m.sku.clone(), location_id: to.clone(), quantity: m.quantity, reserved: 0.0, lot_number: m.lot_number.clone(), expiry_date: None, updated_at: now() });
            }
        }
        drop(stock);
        self.movements.lock().unwrap().push(m);
    }

    pub fn new_movement_id() -> String { format!("mov_{}", uid()) }
    pub fn new_location_id() -> String { format!("loc_{}", uid()) }
    pub fn new_reservation_id() -> String { format!("res_{}", uid()) }
}
