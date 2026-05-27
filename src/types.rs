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
    pub location_type: String, // warehouse, zone, aisle, rack, bin, staging, dock
    pub parent_id: Option<String>,
    pub address: Option<String>,
    pub capacity_units: Option<f64>,   // max units this location can hold
    pub capacity_weight_kg: Option<f64>, // max weight
    pub capacity_volume_m3: Option<f64>, // max volume
    pub used_units: f64,
    pub used_weight_kg: f64,
    pub used_volume_m3: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PickOrder {
    pub id: String,
    pub status: String, // pending, picking, packed, shipped, cancelled
    pub order_reference: String,
    pub lines: Vec<PickLine>,
    pub assigned_to: Option<String>,
    pub wave_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PickLine {
    pub sku: String,
    pub quantity: f64,
    pub from_location: String,
    pub picked_qty: f64,
    pub status: String, // pending, picked, short
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PutawayRule {
    pub id: String,
    pub category: String,       // item category this rule applies to
    pub target_zone: String,    // preferred zone/location
    pub priority: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleCount {
    pub id: String,
    pub location_id: String,
    pub status: String, // scheduled, in_progress, completed
    pub scheduled_date: String,
    pub counted_by: Option<String>,
    pub discrepancies: Vec<Value>,
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wave {
    pub id: String,
    pub name: String,
    pub status: String, // planning, released, in_progress, completed
    pub pick_ids: Vec<String>,
    pub priority: String,
    pub created_at: String,
    pub released_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarcodeLabel {
    pub id: String,
    pub barcode_type: String, // sku, location, lot, shipment, receipt
    pub entity_id: String,
    pub barcode_format: String, // code128, ean13, qr, datamatrix
    pub barcode_value: String,
    pub label_text: Vec<String>,
    pub generated_at: String,
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
