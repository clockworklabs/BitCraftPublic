# BitCraft Deployment and Configuration Guide

## Table of Contents

- [Prerequisites](#prerequisites)
- [Development Setup](#development-setup)
- [Building the Server](#building-the-server)
- [Configuration](#configuration)
- [Running Locally](#running-locally)
- [Static Data Management](#static-data-management)
- [Multi-Region Setup](#multi-region-setup)
- [Production Deployment](#production-deployment)
- [Monitoring and Operations](#monitoring-and-operations)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

**SpacetimeDB**:
- Version: 1.6.0 (exact version required)
- Installation: [https://spacetimedb.com/install](https://spacetimedb.com/install)

```bash
# Install SpacetimeDB CLI
curl -fsSL https://install.spacetimedb.com | bash

# Verify installation
spacetime version
# Should output: spacetime 1.6.0
```

**Rust Toolchain**:
- Version: 1.70 or later
- Installation: [https://rustup.rs/](https://rustup.rs/)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

**Optional Tools**:
- Git (for version control)
- Docker (for containerized deployment)
- PostgreSQL client (for database inspection)

### System Requirements

**Development**:
- CPU: 2+ cores
- RAM: 4GB minimum, 8GB recommended
- Disk: 10GB free space
- OS: Linux, macOS, or Windows with WSL2

**Production** (per region server):
- CPU: 4+ cores (8+ recommended)
- RAM: 16GB minimum, 32GB recommended
- Disk: 100GB+ SSD
- Network: Low latency, high bandwidth
- OS: Linux (Ubuntu 20.04+ recommended)

## Development Setup

### Clone Repository

```bash
git clone https://github.com/clockworklabs/BitCraftPublic.git
cd BitCraftPublic
```

### Verify Project Structure

```bash
tree -L 3 BitCraftServer/
```

Expected structure:
```
BitCraftServer/
├── packages/
│   ├── game/              # Region server module
│   │   ├── src/
│   │   ├── config/
│   │   └── Cargo.toml
│   └── global_module/     # Global server module
│       ├── src/
│       └── Cargo.toml
```

### Install Dependencies

Rust dependencies are managed by Cargo and will be downloaded during build.

```bash
cd BitCraftServer/packages/game
cargo check  # Verify dependencies
```

## Building the Server

### Build Game Module

```bash
cd BitCraftServer/packages/game

# Development build
spacetime build

# Release build (optimized)
spacetime build --release
```

**Output**: `target/wasm32-unknown-unknown/release/bitcraft_spacetimedb.wasm`

### Build Global Module

```bash
cd BitCraftServer/packages/global_module

# Development build
spacetime build

# Release build
spacetime build --release
```

**Output**: `target/wasm32-unknown-unknown/release/bitcraft_spacetimedb.wasm`

### Build Configuration

**Cargo.toml** settings for optimal WASM size:

```toml
[profile.release]
opt-level = 's'       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Single codegen unit
panic = 'abort'       # Smaller panic handler
```

### Troubleshooting Build Issues

**Issue**: "spacetime: command not found"
```bash
# Add to PATH (adjust path if needed)
export PATH="$HOME/.spacetime/bin:$PATH"
```

**Issue**: Rust version mismatch
```bash
# Update Rust
rustup update stable
rustup default stable
```

**Issue**: Build fails with linker errors
```bash
# Install required targets
rustup target add wasm32-unknown-unknown
```

## Configuration

### Configuration Files

Location: `BitCraftServer/packages/game/config/`

**Available Configurations**:
- `default.json` - Default development settings
- `local.example.json` - Template for local development
- `testing.json` - QA/testing environment
- `qa.json` - QA server settings
- `staging.json` - Pre-production environment
- `production.json` - Live game servers

### Creating Local Configuration

```bash
cd BitCraftServer/packages/game/config

# Copy example to local.json
cp local.example.json local.json

# Edit local.json with your settings
nano local.json
```

### Configuration Schema

```json
{
  "env": "dev",
  "cheats": {
    "build_consumes_item_stacks": true,
    "craft_consumes_item_stacks": true,
    "creative_mode": false,
    "dev_pw": "your-dev-password-here"
  },
  "agents": {
    "enabled": true
  },
  "world": {
    "seed": 12345,
    "size": 1000
  }
}
```

### Configuration Parameters

**Environment Settings**:
- `env`: Environment name (`dev`, `testing`, `qa`, `staging`, `production`)
  - Affects cheat availability and logging

**Cheat Settings** (development only):
- `build_consumes_item_stacks`: Whether building costs materials
- `craft_consumes_item_stacks`: Whether crafting costs materials
- `creative_mode`: Unlimited resources
- `dev_pw`: Password for cheat commands

**Agent Settings**:
- `enabled`: Enable background agents (regen, decay, AI)

**World Settings**:
- `seed`: Random seed for world generation
- `size`: World size (number of chunks)

### Environment Variables

SpacetimeDB can be configured via environment variables:

```bash
# Set database directory
export SPACETIMEDB_DATA_DIR=/var/spacetimedb/data

# Set log level
export SPACETIMEDB_LOG_LEVEL=info

# Set max connections
export SPACETIMEDB_MAX_CONNECTIONS=1000
```

## Running Locally

### Start SpacetimeDB

```bash
# Start local SpacetimeDB instance
spacetime start

# With custom log level
spacetime start --log-level debug

# With custom data directory
spacetime start --data-dir ./spacetime-data
```

**Default Settings**:
- HTTP Port: 3000
- WebSocket Port: 3000
- Data Directory: `~/.spacetime`

### Publish Game Module

```bash
cd BitCraftServer/packages/game

# Publish to local instance
spacetime publish bitcraft-region-1 --project-path .

# With specific server
spacetime publish bitcraft-region-1 --server http://localhost:3000
```

**Module Name**: `bitcraft-region-1` (can be any name for local dev)

### Publish Global Module

```bash
cd BitCraftServer/packages/global_module

# Publish global module
spacetime publish bitcraft-global --project-path .
```

### View Module Logs

```bash
# Real-time logs
spacetime logs bitcraft-region-1 --follow

# Filter by level
spacetime logs bitcraft-region-1 --level error

# Last N lines
spacetime logs bitcraft-region-1 --tail 100
```

### Call Reducers Manually

```bash
# Call a reducer via CLI
spacetime call bitcraft-region-1 initialize

# With arguments
spacetime call bitcraft-region-1 cheat_item_stack_grant '{
  "dev_pw": "your-password",
  "item_id": 1,
  "quantity": 100
}'
```

### Query Tables

```bash
# List all tables
spacetime describe bitcraft-region-1

# Query a table
spacetime sql bitcraft-region-1 "SELECT * FROM player_state"

# Export table to CSV
spacetime sql bitcraft-region-1 "SELECT * FROM player_state" --format csv > players.csv
```

### Stop Module

```bash
# Delete module
spacetime delete bitcraft-region-1

# Stop SpacetimeDB
# Ctrl+C or:
pkill spacetime
```

## Static Data Management

### Static Data Overview

Static data defines game content:
- Items (500+ items)
- Buildings (200+ types)
- Recipes (crafting, construction, extraction)
- Enemies (50+ types)
- Biomes (14 biomes)
- Parameters (balance values)

### Data Format

Static data is typically stored in CSV files:

**Example: items.csv**
```csv
id,name,description,item_type,max_stack_size,volume,value
1,Stone,A common stone,Resource,100,1,1
2,Wood,Wooden plank,Resource,50,2,2
3,Iron Ore,Raw iron ore,Resource,50,3,5
```

### Import Process

**Via Reducer**:
```bash
# Stage static data
spacetime call bitcraft-region-1 stage_static_data '{
  "data_version": "v3",
  "csv_data": "<base64-encoded-csv>"
}'
```

**Import Script** (if available):
```bash
# Run import script
./scripts/import_static_data.sh bitcraft-region-1
```

### Verify Import

```bash
# Check item count
spacetime sql bitcraft-region-1 "SELECT COUNT(*) FROM item_desc"

# Check building count
spacetime sql bitcraft-region-1 "SELECT COUNT(*) FROM building_desc"

# List all static data tables
spacetime sql bitcraft-region-1 "
  SELECT table_name
  FROM information_schema.tables
  WHERE table_name LIKE '%_desc'
"
```

### Update Static Data

**Versioning**: Use staged tables for safe updates

1. Upload new data to staging table
2. Validate data integrity
3. Activate new version via reducer
4. Old version remains as backup

**Example**:
```bash
# Stage new data
spacetime call bitcraft-region-1 stage_static_data_v3 '{...}'

# Activate
spacetime call bitcraft-region-1 activate_static_data '{
  "version": "v3"
}'
```

## Multi-Region Setup

### Architecture Overview

```
┌─────────────────┐
│  Global Server  │
│  (Global Module)│
└────────┬────────┘
         │
    ┌────┴────┬────────┬────────┐
    │         │        │        │
┌───▼───┐ ┌──▼───┐ ┌──▼───┐ ┌──▼───┐
│Region0│ │Region1│ │Region2│ │Region3│
│(NW)   │ │(NE)   │ │(SW)   │ │(SE)   │
└───────┘ └───────┘ └───────┘ └───────┘
```

### Region Configuration

**Region Parameters**:
- `region_index`: Region's unique index (0, 1, 2, 3...)
- `region_count_sqrt`: Square root of total regions (2 for 4 regions)

**World Division** (4 regions example):
- Region 0: Northwest quadrant
- Region 1: Northeast quadrant
- Region 2: Southwest quadrant
- Region 3: Southeast quadrant

### Deploy Global Server

```bash
cd BitCraftServer/packages/global_module

# Build and publish
spacetime build --release
spacetime publish bitcraft-global --server https://global.example.com
```

### Deploy Region Servers

**Region 0**:
```bash
cd BitCraftServer/packages/game

# Set region configuration
export REGION_INDEX=0
export REGION_COUNT_SQRT=2
export GLOBAL_SERVER=https://global.example.com

# Publish
spacetime publish bitcraft-region-0 --server https://region-0.example.com
```

**Repeat for all regions** (1, 2, 3...)

### Configure Inter-Module Communication

**Global Server Identity**:
Each region needs to know global server's identity for message routing.

**Setup**:
1. Get global server identity:
   ```bash
   spacetime identity list --server https://global.example.com
   ```

2. Configure region to trust global identity:
   ```bash
   spacetime call bitcraft-region-0 set_global_identity '{
     "identity": "<global-identity-hash>"
   }'
   ```

### World Generation for Regions

**Coordinate Assignment**:
```rust
fn get_region_for_location(location: SmallHexTile, region_count_sqrt: u32) -> u32 {
    // Determine which region owns this coordinate
    let region_x = if location.q >= 0 { 1 } else { 0 };
    let region_y = if location.r >= 0 { 1 } else { 0 };
    region_y * region_count_sqrt + region_x
}
```

**Generate World**:
1. Generate full world offline
2. Partition chunks by region
3. Upload chunks to respective regions:
   ```bash
   # Upload to region 0
   spacetime call bitcraft-region-0 insert_terrain_chunk '{...}'
   ```

### Test Cross-Region Transfer

```bash
# Move player from region 0 to region 1
spacetime call bitcraft-region-0 player_region_crossover '{
  "destination_region": 1,
  "destination_x": 100.0,
  "destination_z": 100.0
}'
```

## Production Deployment

### Infrastructure Requirements

**Per Region Server**:
- Load balancer for HA
- Persistent storage (SSD)
- Backup system
- Monitoring/alerting
- DDoS protection

**Global Server**:
- High availability (primary + standby)
- Fast storage (NVMe SSD)
- Low latency network to regions
- Redundant network connections

### Deployment Checklist

**Pre-Deployment**:
- [ ] Build release binaries (`--release`)
- [ ] Validate static data integrity
- [ ] Test all reducers
- [ ] Load test with expected player count
- [ ] Security audit
- [ ] Backup plan established

**Deployment**:
- [ ] Deploy global server first
- [ ] Wait for global server healthy
- [ ] Deploy region servers sequentially
- [ ] Configure inter-module communication
- [ ] Upload static data
- [ ] Generate and upload world
- [ ] Enable agents
- [ ] Smoke test all systems

**Post-Deployment**:
- [ ] Monitor logs for errors
- [ ] Check agent execution
- [ ] Verify player sign-in
- [ ] Test cross-region transfers
- [ ] Monitor performance metrics
- [ ] Enable external access

### Security Hardening

**Authentication**:
```bash
# Disable public access (production)
spacetime acl bitcraft-region-1 --mode private

# Add authorized identities
spacetime acl bitcraft-region-1 --add <identity-hash>
```

**Rate Limiting**:
Configure SpacetimeDB rate limits:
```bash
# Limit reducer calls per second
export SPACETIMEDB_RATE_LIMIT_RPS=100

# Limit connections per IP
export SPACETIMEDB_MAX_CONNECTIONS_PER_IP=10
```

**Firewall Rules**:
```bash
# Allow only WebSocket traffic
ufw allow 3000/tcp

# Block direct database access
ufw deny 5432/tcp
```

### Backup Strategy

**Automated Backups**:
```bash
#!/bin/bash
# backup.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups/bitcraft"

# Export database
spacetime database export bitcraft-region-1 > "$BACKUP_DIR/region-1_$DATE.sql"

# Compress
gzip "$BACKUP_DIR/region-1_$DATE.sql"

# Retain last 30 days
find $BACKUP_DIR -name "*.sql.gz" -mtime +30 -delete
```

**Schedule with cron**:
```cron
# Backup every 6 hours
0 */6 * * * /scripts/backup.sh
```

### Restore from Backup

```bash
# Stop module
spacetime delete bitcraft-region-1

# Restore from backup
gunzip -c /backups/bitcraft/region-1_20260123_120000.sql.gz | \
  spacetime database import bitcraft-region-1

# Restart
spacetime publish bitcraft-region-1 --project-path ./packages/game
```

## Monitoring and Operations

### Logging

**Log Levels**:
- `error`: Critical errors requiring immediate attention
- `warn`: Warning conditions
- `info`: Informational messages
- `debug`: Detailed debug information
- `trace`: Very detailed trace information

**Configure Logging**:
```bash
# Set log level
export SPACETIMEDB_LOG_LEVEL=info

# Enable structured logging
export SPACETIMEDB_LOG_FORMAT=json
```

**Centralized Logging**:
```bash
# Forward logs to syslog
spacetime start --log-output syslog

# Or to file
spacetime start --log-output /var/log/spacetimedb/bitcraft.log
```

### Performance Metrics

**Key Metrics**:
- Reducer call rate (calls/second)
- Reducer latency (ms per call)
- Active connections
- Database size
- Memory usage
- CPU usage

**Query Metrics**:
```bash
# Get module statistics
spacetime metrics bitcraft-region-1

# Monitor in real-time
watch -n 5 'spacetime metrics bitcraft-region-1'
```

**Custom Metrics**:
Add logging to reducers:
```rust
#[spacetimedb::reducer]
pub fn my_reducer(ctx: &ReducerContext) -> Result<(), String> {
    let start = std::time::Instant::now();

    // ... reducer logic ...

    let duration = start.elapsed();
    log::info!("Reducer completed in {:?}", duration);

    Ok(())
}
```

### Health Checks

**Endpoint Health**:
```bash
# Check if SpacetimeDB is responsive
curl http://localhost:3000/health
```

**Module Health**:
```bash
# Verify module is running
spacetime list | grep bitcraft-region-1
```

**Automated Health Check Script**:
```bash
#!/bin/bash
# health_check.sh

MODULES=("bitcraft-global" "bitcraft-region-0" "bitcraft-region-1")

for module in "${MODULES[@]}"; do
  if spacetime list | grep -q "$module"; then
    echo "$module: OK"
  else
    echo "$module: FAILED"
    # Alert or restart
    spacetime publish $module --project-path ./packages/game
  fi
done
```

### Alerting

**Alert Conditions**:
- Module unavailable
- High error rate in logs
- High reducer latency (>1000ms)
- Database size exceeding threshold
- Memory usage >80%
- CPU usage >90%

**Example: Email Alert on Error**:
```bash
# monitor_errors.sh
ERROR_COUNT=$(spacetime logs bitcraft-region-1 --level error --tail 100 | wc -l)

if [ $ERROR_COUNT -gt 10 ]; then
  echo "High error count: $ERROR_COUNT" | \
    mail -s "BitCraft Alert: High Error Rate" admin@example.com
fi
```

## Troubleshooting

### Common Issues

**Issue: Module won't publish**
```bash
# Check SpacetimeDB is running
spacetime list

# Check build succeeded
cd packages/game
spacetime build

# Verify WASM output exists
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Try with verbose output
spacetime publish bitcraft-test --project-path . --verbose
```

**Issue: Reducer calls fail**
```bash
# Check module is running
spacetime list | grep bitcraft-region-1

# Check logs for errors
spacetime logs bitcraft-region-1 --level error

# Verify reducer exists
spacetime describe bitcraft-region-1 | grep <reducer-name>

# Test with simple reducer
spacetime call bitcraft-region-1 initialize
```

**Issue: Players can't sign in**
```bash
# Check user state
spacetime sql bitcraft-region-1 "SELECT * FROM user_state WHERE identity = '<identity>'"

# Check queue
spacetime sql bitcraft-region-1 "SELECT * FROM user_state WHERE can_sign_in = false"

# Enable sign in
spacetime call bitcraft-region-1 admin_allow_sign_in '{
  "identity": "<identity>"
}'
```

**Issue: Agents not running**
```bash
# Check agents enabled in config
cat config/local.json | grep agents

# Check agent schedules
spacetime sql bitcraft-region-1 "SELECT * FROM player_regen_schedule"

# Manually trigger agent
spacetime call bitcraft-region-1 schedule_player_regen_agent '{
  "initial_delay": 1000
}'
```

**Issue: High memory usage**
```bash
# Check database size
spacetime database size bitcraft-region-1

# Identify large tables
spacetime sql bitcraft-region-1 "
  SELECT table_name,
         pg_size_pretty(pg_total_relation_size(table_name::regclass))
  FROM information_schema.tables
  WHERE table_schema = 'public'
  ORDER BY pg_total_relation_size(table_name::regclass) DESC
  LIMIT 10
"

# Clean up old data
spacetime call bitcraft-region-1 chat_cleanup_agent # If available
```

### Debug Mode

**Enable Debug Logging**:
```bash
export SPACETIMEDB_LOG_LEVEL=debug
spacetime start

# Or for specific module
spacetime logs bitcraft-region-1 --level debug --follow
```

**Add Debug Prints to Reducers**:
```rust
#[spacetimedb::reducer]
pub fn debug_reducer(ctx: &ReducerContext) -> Result<(), String> {
    log::debug!("Debug reducer called by {:?}", ctx.sender);
    log::debug!("Current timestamp: {:?}", ctx.timestamp);

    // ... reducer logic ...

    Ok(())
}
```

### Performance Profiling

**Identify Slow Reducers**:
```bash
# Filter logs for slow operations
spacetime logs bitcraft-region-1 --follow | grep -E "duration.*[5-9][0-9]{2}ms"
```

**Database Query Performance**:
```bash
# Enable query logging in SpacetimeDB
export SPACETIMEDB_LOG_QUERIES=true

# Analyze slow queries
spacetime logs bitcraft-region-1 | grep "slow query"
```

### Getting Help

**Resources**:
- SpacetimeDB Documentation: [https://spacetimedb.com/docs](https://spacetimedb.com/docs)
- SpacetimeDB Discord: Community support
- GitHub Issues: Bug reports and feature requests

**Reporting Issues**:
Include:
1. SpacetimeDB version (`spacetime version`)
2. Module build output
3. Relevant logs
4. Steps to reproduce
5. Expected vs actual behavior

## Summary

This guide covers:
- ✅ Development environment setup
- ✅ Building and publishing modules
- ✅ Configuration management
- ✅ Local development workflow
- ✅ Static data import
- ✅ Multi-region deployment
- ✅ Production hardening
- ✅ Monitoring and operations
- ✅ Troubleshooting common issues

For more detailed information, refer to other documentation:
- **[Overview](overview.md)** - Project overview and concepts
- **[Architecture](architecture.md)** - System architecture
- **[Data Models](data-models.md)** - Database schemas
- **[Reducers API](reducers-api.md)** - API reference
- **[Game Systems](game-systems.md)** - Game mechanics
