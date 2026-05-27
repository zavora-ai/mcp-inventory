# Inventory & Warehouse MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-inventory.svg)](https://crates.io/crates/mcp-inventory)
[![Docs.rs](https://docs.rs/mcp-inventory/badge.svg)](https://docs.rs/mcp-inventory)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://enterprise.adk-rust.com)

Complete inventory and warehouse management for [ADK-Rust Enterprise](https://enterprise.adk-rust.com) agents. Provides 38 MCP tools covering the full warehouse lifecycle — stock management, serialized tracking, RFID, QR codes, pick/pack/ship, wave planning, putaway, cycle counts, BOM, and space utilization. **Zero configuration, no external dependencies.**

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        mcp-inventory (38 tools)                          │
├────────────┬────────────┬────────────┬────────────┬─────────────────────┤
│  Catalog   │   Stock    │  Warehouse │ Serialized │   Identification    │
│  & BOM     │ Movements  │    Ops     │  Tracking  │   & Labeling        │
├────────────┼────────────┼────────────┼────────────┼─────────────────────┤
│ Items      │ Receive    │ Pick/Pack  │ Serial Reg │ Barcode (Code128)   │
│ Locations  │ Issue      │ Ship       │ Serial Move│ QR Codes            │
│ BOM        │ Transfer   │ Waves      │ Chain of   │ RFID (EPC)          │
│ Reorder    │ Adjust     │ Putaway    │ Custody    │ Batch Labels        │
│            │ Reserve    │ Cycle Count│ Location   │                     │
│            │            │ Space Mgmt │ Scan       │                     │
└────────────┴────────────┴────────────┴────────────┴─────────────────────┘
```

## Key Principles

- **Full lifecycle** — from goods receipt through storage, picking, packing, shipping, and returns.
- **Serialized tracking** — individual unit tracking with full chain of custody history.
- **RFID-ready** — register tags, process bulk reader scans, detect missing inventory.
- **QR code generation** — encode serial, SKU, location, and shipment data for mobile scanning.
- **Space-aware** — locations have capacity limits (units, weight, volume) with utilization reporting.
- **Wave planning** — batch multiple pick orders for efficient warehouse operations.
- **Zero configuration** — starts immediately with no external dependencies or API keys.

## Tools (38)

### Item Catalog

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `item_upsert` | Add/update item (SKU, name, category, unit, reorder point, cost) | write |
| `item_list` | List all items in catalog | read-only |

### Locations & Space

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `location_create` | Create warehouse/zone/aisle/rack/bin with capacity limits | write |
| `location_list` | List all locations | read-only |
| `space_utilization` | Capacity vs used report (units, weight, volume) | read-only |

### Stock Movements

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `stock_receive` | Receive goods into location (from supplier, production, return) | write |
| `stock_issue` | Issue stock from location (sales, consumption, dispatch) | write |
| `stock_transfer` | Transfer between locations | write |
| `stock_adjust` | Adjust quantity (cycle count correction, damage write-off) | write (approval) |
| `stock_check` | Check stock level (total, reserved, available) | read-only |
| `reorder_alerts` | Items below reorder point with suggested order quantities | read-only |
| `movement_history` | Full movement audit trail for a SKU | read-only |

### Reservations

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `stock_reserve` | Reserve stock for an order (prevents overselling) | write |
| `stock_release` | Release a reservation (cancel, expiry) | write |

### Bill of Materials

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `bom_set` | Define components needed to build a parent item | write |
| `bom_check` | Check if enough components are available to build N units | read-only |

### Pick / Pack / Ship

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `pick_create` | Create pick order with line items and location allocation | write |
| `pick_confirm` | Confirm picked quantities (detect shorts) | write |
| `pick_ship` | Mark as shipped (issues stock from locations) | write |
| `pick_list` | List all pick orders | read-only |

### Wave Planning

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `wave_create` | Batch multiple pick orders into a wave | write |
| `wave_release` | Release wave (moves all picks to "picking" status) | write |
| `wave_complete` | Mark wave as completed | write |
| `wave_list` | List all waves | read-only |

### Putaway

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `putaway_rule_create` | Define preferred location by item category | write |
| `putaway_suggest` | Suggest optimal bin for an item based on rules and space | read-only |

### Cycle Counts

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `cycle_count_schedule` | Schedule a count for a location | write |
| `cycle_count_complete` | Submit actual counts, auto-detects discrepancies | write |

### Serialized Warehousing

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `serial_register` | Register individual unit (serial number, lot, RFID, auto-QR) | write |
| `serial_move` | Move serialized item (full chain of custody) | write |
| `serial_lookup` | Look up serial — full history, location, status | read-only |
| `serial_scan_location` | List all serialized items at a location | read-only |

### RFID

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `rfid_register` | Register RFID EPC tag, link to serial/SKU | write |
| `rfid_bulk_read` | Process reader scan — found, unknown, and missing detection | write |
| `rfid_lookup` | Look up tag by EPC | read-only |

### QR Codes & Labels

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `qr_generate` | Generate QR payload for serial/SKU/location/shipment | write |
| `label_generate` | Generate barcode label (code128, EAN-13, QR, DataMatrix) | write |
| `label_batch` | Batch generate labels for multiple entities | write |

## Installation

### From crates.io

```bash
cargo install mcp-inventory
```

### Build from source

```bash
git clone https://github.com/zavora-ai/mcp-inventory
cd mcp-inventory
cargo build --release
```

### Claude Desktop

```json
{
  "mcpServers": {
    "inventory": { "command": "mcp-inventory" }
  }
}
```

### Kiro

Add to `.kiro/settings/mcp.json`:

```json
{
  "mcpServers": {
    "inventory": { "command": "mcp-inventory" }
  }
}
```

### Cursor / Windsurf / Codex

```json
{
  "mcpServers": {
    "inventory": { "command": "mcp-inventory" }
  }
}
```

## Quick Start

### 1. Add items and locations

```json
{"name": "item_upsert", "arguments": {"sku": "LAPTOP-PRO", "name": "Laptop Pro 16\"", "category": "electronics", "unit": "each", "reorder_point": 20, "reorder_qty": 100, "cost": 850, "currency": "USD"}}
```

```json
{"name": "location_create", "arguments": {"name": "Warehouse A - Bin A3-07", "location_type": "bin", "capacity_units": 50, "capacity_weight_kg": 500}}
```

### 2. Receive stock

```json
{"name": "stock_receive", "arguments": {"sku": "LAPTOP-PRO", "quantity": 30, "location_id": "loc_abc123", "actor": "receiving_clerk", "reference": "PO-2026-0142", "lot_number": "LOT-2026Q2"}}
```

### 3. Register serialized items with RFID

```json
{"name": "serial_register", "arguments": {"serial_number": "SN-2026-00142", "sku": "LAPTOP-PRO", "location_id": "loc_abc123", "lot_number": "LOT-2026Q2", "rfid_tag": "E200001234567890"}}
```

**Response:**
```json
{"status": "registered", "serial_number": "SN-2026-00142", "qr_code": "QR:SN=SN-2026-00142&SKU=LAPTOP-PRO&LOC=loc_abc123"}
```

### 4. Create pick order and wave

```json
{"name": "pick_create", "arguments": {"order_reference": "ORD-8821", "items": [{"sku": "LAPTOP-PRO", "quantity": 2, "location_id": "loc_abc123"}], "default_location": "loc_abc123", "assigned_to": "picker_john"}}
```

```json
{"name": "wave_create", "arguments": {"name": "Morning Wave", "pick_ids": ["pick_abc", "pick_def", "pick_ghi"], "priority": "high"}}
```

### 5. RFID bulk scan (detect missing inventory)

```json
{"name": "rfid_bulk_read", "arguments": {"location_id": "loc_abc123", "epcs": ["E200001234567890", "E200009999999999"]}}
```

**Response:**
```json
{"location_id": "loc_abc123", "scanned": 2, "found": 1, "unknown": 1, "missing": 0, "found_tags": [{"epc": "E200001234567890", "serial": "SN-2026-00142"}], "unknown_epcs": ["E200009999999999"], "missing_tags": []}
```

### 6. Track serialized item movement

```json
{"name": "serial_move", "arguments": {"serial_number": "SN-2026-00142", "to_location": "STAGING-DOCK-1", "actor": "picker_john", "event_type": "picked", "reference": "ORD-8821"}}
```

### 7. Generate QR code for shipping label

```json
{"name": "qr_generate", "arguments": {"entity_type": "serial", "entity_id": "SN-2026-00142", "extra_data": {"order": "ORD-8821", "destination": "Customer XYZ"}}}
```

## Location Hierarchy

```
Warehouse A (warehouse)
├── Zone 1 - Electronics (zone)
│   ├── Aisle A (aisle)
│   │   ├── Rack A1 (rack)
│   │   │   ├── Bin A1-01 (bin) [capacity: 100 units, 200kg]
│   │   │   ├── Bin A1-02 (bin) [capacity: 100 units, 200kg]
│   │   │   └── ...
│   │   └── Rack A2 (rack)
│   └── Aisle B (aisle)
├── Zone 2 - Bulk Storage (zone)
├── Staging Area (staging)
└── Dock 1 (dock)
```

## Serialized Item Lifecycle

```
serial_register → in_stock
    │
    ├── serial_move (event: "picked") → in_stock (at staging)
    │       │
    │       ├── serial_move (event: "shipped") → shipped
    │       │       │
    │       │       └── serial_move (event: "returned") → in_stock
    │       │
    │       └── serial_move (event: "scrapped") → scrapped
    │
    └── rfid_bulk_read → detect if missing from expected location
```

## RFID Operations

| Operation | What happens |
|-----------|-------------|
| `rfid_register` | Link EPC to serial/SKU, set initial location |
| `rfid_bulk_read` | Reader scans area → server compares against expected tags |
| **Found** | Tag is at expected location — update `last_read_at` |
| **Unknown** | Tag not in system — flag for investigation |
| **Missing** | Tag expected but not scanned — potential theft/misplacement |

## Barcode Formats

| Format | Use Case | Example Value |
|--------|----------|---------------|
| `code128` | General purpose, high density | `SKU-LAPTOP-PRO` |
| `ean13` | Retail products | `5901234123457` |
| `qr` | Mobile scanning, rich data | `SN=X&SKU=Y&LOC=Z` |
| `datamatrix` | Small items, PCBs | Compact binary |
| `rfid_epc` | RFID tags (96-bit) | `E200001234567890` |

## Stock Movement Types

| Type | From | To | Use Case |
|------|------|-----|----------|
| `receive` | — | Location | Goods receipt from supplier |
| `issue` | Location | — | Sales, consumption, dispatch |
| `transfer` | Location A | Location B | Internal movement |
| `adjust` | — | — | Cycle count correction, write-off |

## Configuration

### Environment Variables

| Variable | Required | Purpose |
|----------|:--------:|---------|
| `RUST_LOG` | No | Log level (default: `info`) |

No API keys or external services needed. All data is stored in-memory.

### MCP Server Manifest

```toml
server_id = "mcp_inventory"
display_name = "Inventory"
version = "1.3.0"
domain = "inventory"
risk_level = "medium"
writes_allowed = "gated"
governance_gates = ["stock_movement_audit"]
```

## Use Cases

### Manufacturing
```
bom_set (define components for finished goods)
stock_receive (raw materials from supplier)
bom_check (can we build 100 units?)
stock_issue (consume components)
serial_register (each finished unit gets serial + RFID)
```

### E-Commerce Fulfillment
```
stock_receive (bulk from supplier)
stock_reserve (customer places order)
pick_create → wave_create → wave_release (batch fulfillment)
pick_confirm → pick_ship (dispatch)
serial_move (event: "shipped", reference: order_id)
```

### Pharmaceutical / Regulated
```
serial_register (each unit serialized per regulation)
rfid_register (cold chain monitoring)
serial_move (full chain of custody for audit)
cycle_count_schedule → cycle_count_complete (regulatory compliance)
qr_generate (patient-facing verification)
```

### Retail / Multi-Store
```
location_create (store_1, store_2, warehouse)
stock_transfer (warehouse → store)
reorder_alerts (per-location thresholds)
rfid_bulk_read (daily inventory verification)
space_utilization (optimize shelf allocation)
```

## Error Codes

| Code | Meaning |
|------|---------|
| `INSUFFICIENT_STOCK` | Not enough available stock for operation |
| `SERIAL_NOT_FOUND` | Serial number not registered |
| `RFID_NOT_FOUND` | EPC tag not in system |
| `PICK_NOT_FOUND` | Pick order ID doesn't exist |
| `WAVE_NOT_FOUND` | Wave ID doesn't exist |
| `BOM_NOT_FOUND` | No BOM defined for parent SKU |
| `RESERVATION_NOT_FOUND` | Reservation ID doesn't exist |
| `CYCLE_COUNT_NOT_FOUND` | Cycle count ID doesn't exist |

## Documentation

| Document | Description |
|----------|-------------|
| [mcp-server.toml](mcp-server.toml) | ADK-Rust Enterprise registry manifest |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [Rust Docs](https://docs.rs/mcp-inventory) | Generated API documentation |

## Contributing

Contributions welcome. Priority areas:
- Persistent storage backend (PostgreSQL/SQLite)
- Expiry date management (FEFO picking)
- Multi-warehouse transfer orders
- Inventory valuation (FIFO, LIFO, weighted average)
- Integration with mcp-logistics for shipping
- Barcode image generation (SVG/PNG)

## Contributors

<!-- ALL-CONTRIBUTORS-LIST:START -->
| [<img src="https://github.com/jkmaina.png" width="80px;" alt=""/><br /><sub><b>James Karanja Maina</b></sub>](https://github.com/jkmaina) |
|:---:|
<!-- ALL-CONTRIBUTORS-LIST:END -->

## License

Apache-2.0 — see [LICENSE](LICENSE) for details.

---

Part of the [ADK-Rust Enterprise](https://enterprise.adk-rust.com) MCP server ecosystem.

Built with ❤️ by [Zavora AI](https://zavora.ai)

## Registry Compliance

This server implements the [ADK MCP SDK](https://crates.io/crates/adk-mcp-sdk) contract:

- **HealthCheck** — async health probe for registry monitoring
- **mcp-server.toml** — manifest declaring tools, risk classes, and credentials
- **Structured tracing** — `RUST_LOG` env-filter for observability
- **Audit trail** — every stock movement logged with actor, timestamp, and reference
