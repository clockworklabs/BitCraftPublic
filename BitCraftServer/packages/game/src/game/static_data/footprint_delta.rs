use crate::messages::static_data::{FootprintTile, FootprintType};

impl FootprintTile {
    pub fn is_compatible(existing: &FootprintType, new: &FootprintType) -> bool {
        match (*existing, *new) {
            // Platforms only conflict with other platforms. Whether a building may stand on a given platform is checked
            // separately (FloatingPlatformDesc::supports_buildings).
            (FootprintType::Platform, FootprintType::Platform) => false,
            (FootprintType::Platform, _) => true,
            (_, FootprintType::Platform) => *existing != FootprintType::Hitbox && *existing != FootprintType::Walkable,
            (FootprintType::Hitbox, _) => false,
            (FootprintType::Walkable, _) => false,
            (FootprintType::WalkableResource, _) => true,
            (FootprintType::Perimeter, FootprintType::Hitbox) => false,
            (FootprintType::Perimeter, FootprintType::Walkable) => false,
            (FootprintType::Perimeter, FootprintType::Perimeter) => true,
            (FootprintType::Perimeter, FootprintType::WalkableResource) => true,
        }
    }
}
