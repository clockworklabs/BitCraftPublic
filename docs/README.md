# BitCraft Server Documentation

Welcome to the BitCraft Server technical documentation. This documentation provides comprehensive specifications for the BitCraft server-side implementation.

## About BitCraft

BitCraft is a community sandbox MMORPG developed by Clockwork Labs. The server is built on [SpacetimeDB](https://spacetimedb.com), a real-time, reactive backend platform designed for multiplayer game development.

This documentation covers the open-source server implementation, including game logic, state management, and server-side systems.

## Documentation Structure

- **[Overview](overview.md)** - Project overview, structure, and getting started
- **[Architecture](architecture.md)** - System architecture, design patterns, and unique implementations
- **[Data Models](data-models.md)** - Database schema, tables, and data structures
- **[Reducers API](reducers-api.md)** - Complete reducer (API endpoint) reference
- **[Game Systems](game-systems.md)** - Core game systems and mechanics documentation
- **[Deployment](deployment.md)** - Deployment, configuration, and operations guide

## Quick Start

### Prerequisites

- [SpacetimeDB CLI](https://spacetimedb.com/install) (version 1.6.0)
- Rust toolchain
- Git

### Building the Server

```bash
cd BitCraftServer/packages/game
spacetime build
```

### Local Development

See [Deployment Guide](deployment.md) for detailed setup instructions.

## Key Technologies

- **Language**: Rust
- **Database/Platform**: SpacetimeDB 1.6.0
- **Architecture**: Distributed multi-region server system
- **Coordinate System**: Hexagonal tile-based world

## Project Statistics

- **Total Reducers**: 675+ API endpoints
- **Database Tables**: 200+ SpacetimeDB tables
- **Lines of Code**: ~150,000+
- **Background Agents**: 23 scheduled tasks
- **Game Systems**: 10+ major systems (Combat, Crafting, Building, etc.)

## Contributing

Please see [CONTRIBUTING.md](../CONTRIBUTING.md) in the root directory for contribution guidelines.

## License

The BitCraft source code is licensed under the Apache 2.0 license. See [LICENSE](../LICENSE) for details.

## Additional Resources

- [BitCraft Website](https://bitcraftonline.com)
- [SpacetimeDB Documentation](https://spacetimedb.com/docs)
- [SpacetimeDB GitHub](https://github.com/clockworklabs/SpacetimeDB)
- [Discord Community](https://discord.com/invite/bitcraft)
