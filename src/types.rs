use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub unit: String, // each, kg, litre, box, pallet
    pub reorder_point: f64,
    pub reorder_qty: f64,
    pub cost: f64,
    pub currency: String,
    pub attributes: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub location_type: String, // warehouse, store, bin, zone, transit
    pub parent_id: Option<String>,
    pub address: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockLevel {
    pub sku: String,
    pub location_id: String,
    pub quantity: f64,
    pub reserved: f64,
    pub lot_number: Option<String>,
    pub expiry_date: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: String,
    pub movement_type: String, // receive, issue, transfer, adjust, return
    pub sku: String,
    pub quantity: f64,
    pub from_location: Option<String>,
    pub to_location: Option<String>,
    pub reference: String,
    pub actor: String,
    pub lot_number: Option<String>,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BomEntry {
    pub parent_sku: String,
    pub component_sku: String,
    pub quantity: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reservation {
    pub id: String,
    pub sku: String,
    pub location_id: String,
    pub quantity: f64,
    pub reference: String, // order_id, work_order_id
    pub expires_at: Option<String>,
    pub created_at: String,
}
