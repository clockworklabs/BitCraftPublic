use super::resource_definition::ResourceDefinition;

#[derive(Debug)]
pub struct ResourcesMapDefinition {
    pub seed: i32,
    pub resources: Vec<ResourceDefinition>,
}

impl ResourcesMapDefinition {
    pub fn count(&self) -> i32 {
        self.resources.len() as i32
    }

    pub fn get_resource(&self, index: i32) -> Option<&ResourceDefinition> {
        if self.count() == 0 {
            return None;
        }

        let index = index.clamp(0, self.count() - 1);
        Some(&self.resources[index as usize])
    }
}
