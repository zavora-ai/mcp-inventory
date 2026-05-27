# Changelog

## [1.3.0] - 2026-05-27

### Added — Serialized Warehousing, RFID, QR Codes
- `serial_register` — register individual units with serial number, auto-QR, optional RFID link
- `serial_move` — move serialized items with full chain of custody (received → picked → shipped → returned → scrapped)
- `serial_lookup` — full history and status of a serial number
- `serial_scan_location` — list all serialized items at a location
- `rfid_register` — register RFID EPC tag, link to serial/SKU
- `rfid_bulk_read` — process reader scan with found/unknown/missing detection
- `rfid_lookup` — look up tag by EPC
- `qr_generate` — generate QR payload for serial/SKU/location/shipment with extra data encoding

## [1.2.0] - 2026-05-27

### Added — Wave Planning & Barcode Labels
- `wave_create` — batch multiple pick orders into a wave for efficient picking
- `wave_release` — release wave (moves all picks to "picking" status)
- `wave_complete` — mark wave as completed
- `wave_list` — list all waves with status
- `label_generate` — generate barcode label (code128, EAN-13, QR, DataMatrix)
- `label_batch` — batch generate labels for multiple entities

## [1.1.0] - 2026-05-27

### Added — WMS Features
- `pick_create` — create pick order with line items and location allocation
- `pick_confirm` — confirm picked quantities (detect shorts)
- `pick_ship` — mark as shipped (issues stock from locations)
- `pick_list` — list all pick orders
- `putaway_rule_create` — define preferred location by item category
- `putaway_suggest` — suggest optimal bin based on rules and available space
- `cycle_count_schedule` — schedule a count for a location
- `cycle_count_complete` — submit actual counts with discrepancy detection
- `space_utilization` — capacity vs used report (units, weight, volume)
- Location capacity fields: `capacity_units`, `capacity_weight_kg`, `capacity_volume_m3`

## [1.0.0] - 2026-05-27

### Added — Core Inventory
- `item_upsert` — add/update inventory items with reorder points
- `item_list` — list all items
- `location_create` — create warehouse/zone/bin locations
- `location_list` — list locations
- `stock_receive` — receive goods into location
- `stock_issue` — issue stock (validates availability)
- `stock_transfer` — transfer between locations
- `stock_adjust` — adjust quantity (cycle count, write-off)
- `stock_check` — check stock level (total, reserved, available, below reorder)
- `reorder_alerts` — items below reorder point with suggested order quantities
- `stock_reserve` — reserve stock for orders (prevents overselling)
- `stock_release` — release reservations
- `bom_set` — define bill of materials
- `bom_check` — check component availability to build N units
- `movement_history` — full movement audit trail
