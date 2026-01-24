use spacetimedb::{Identity, Timestamp};

use crate::{
    i18n,
    messages::components::{ChatChannel, ChatMessageState},
};

use super::generic::HubItemType;

#[spacetimedb::table(name = user_region_state, index(name = region_id, btree(columns =[region_id])), public)]
pub struct UserRegionState {
    #[primary_key]
    pub identity: Identity,
    pub region_id: u8,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PlayerVoteType")]
#[repr(i32)]
pub enum PlayerVoteType {
    JoinEmpire,
    SubmitEmpire,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PlayerVoteAnswer")]
#[repr(i32)]
pub enum PlayerVoteAnswer {
    None,
    No,
    Yes,
}

#[spacetimedb::table(name = player_vote_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct PlayerVoteState {
    #[primary_key]
    pub entity_id: u64,

    pub vote_type: PlayerVoteType,
    pub initiator_entity_id: u64,
    pub participants_entity_id: Vec<u64>,
    pub answers: Vec<PlayerVoteAnswer>,
    pub initiator_name: String,
    pub pass_threshold: f32,
    pub outcome: PlayerVoteAnswer,
    //DAB Note: These arguments should be in separate tables (one per vote_type). This will allow them to have better names and
    //  make sure that votes don't get mis-interpreted (right now there's nothing stopping you from using JoinEmpire vote as if it were TeleportRequest)
    pub argument1: u64,
    pub argument2: u64,
    pub outcome_str: String,
}

#[spacetimedb::table(name = player_shard_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct PlayerShardState {
    #[primary_key]
    pub entity_id: u64,
    pub shards: u32,
    pub last_shard_claim: i32, // Timestamp doesn't implement Default. Also, might as well save a few bytes since we don't care about milliseconds
}

#[spacetimedb::table(name = granted_hub_item_state, index(name = identity_and_item_id, btree(columns = [identity, item_id])))]
#[derive(Clone, Debug)]
pub struct GrantedHubItemState {
    #[primary_key]
    #[auto_inc]
    pub entity_id: u64,
    pub identity: Identity,
    pub item_type: HubItemType,
    pub item_id: i32,
    pub balance: u32,
}

#[spacetimedb::table(name = player_developer_notification_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct PlayerDeveloperNotificationState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    // Note that C-style enums count as size/align of 1,
    // regardless of declared `repr` in Rust.
    #[primary_key]
    pub entity_id: u64,
    pub title: String,
    pub message: String,
}

#[spacetimedb::table(name = direct_message_state, public,
    index(name = sender_entity_id, btree(columns = [sender_entity_id])),
    index(name = receiver_entity_id, btree(columns = [receiver_entity_id])))]
#[derive(Clone, Debug)]
// NOT USED ANYMORE.
pub struct DirectMessageState {
    #[primary_key]
    pub entity_id: u64,
    pub username: String,
    pub title_id: i32,
    pub sender_entity_id: u64,
    pub receiver_entity_id: u64,
    pub text: String,
    pub timestamp: i32,
    #[default(None::<String>)]
    pub language_code: Option<String>,
}

// TODO: We can get rid of this once we fix RLS (or use Views) and use ChatMessageState for DMs
impl DirectMessageState {
    /// Convert a DM record into a ChatMessageState:
    ///  - channel_id = global
    ///  - owner_entity_id = sender_entity_id
    ///  - target_id = receiver_entity_id
    pub fn into_chat_message_state(self) -> ChatMessageState {
        ChatMessageState {
            entity_id: self.entity_id,
            //username: self.username, //I18N
            title_id: self.title_id,
            channel_id: ChatChannel::Global as i32, // to keep consistent sent DM's
            target_id: self.receiver_entity_id,
            text: self.text,
            timestamp: self.timestamp,
            owner_entity_id: self.sender_entity_id,
            //language_code: self.language_code //I18N
            username: i18n::dont_reformat(format!("{}/{}", self.language_code.unwrap_or("".to_string()), self.username)), //I18N
        }
    }
}

#[spacetimedb::table(name = chat_channel_state, public,
    index(name = lowercase_name, btree(columns = [lowercase_name]))
)]
#[derive(Clone, Debug)]
pub struct ChatChannelState {
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub name: String,
    pub lowercase_name: String,
    pub description: String,
    pub visibility: ChatChannelVisibility,
}

#[spacetimedb::table(name = chat_channel_permission_state, public,
    index(name = chat_channel_entity_id, btree(columns = [chat_channel_entity_id])),
    index(name = player_entity_id, btree(columns = [player_entity_id])),
    index(name = chat_channel_and_player_entity_id, btree(columns = [chat_channel_entity_id, player_entity_id])),
    index(name = identity, btree(columns = [identity])),
    index(name = rank, btree(columns = [rank]))
)]
#[derive(Clone, Debug)]
pub struct ChatChannelPermissionState {
    #[primary_key]
    pub entity_id: u64,
    pub chat_channel_entity_id: u64,
    pub player_entity_id: u64,
    pub identity: Identity,
    pub rank: i32,
}

pub const MAX_CHAT_CHANNELS_PER_PLAYER: usize = 5; // TODO: Move to table
pub const MAX_MEMBERS_PER_CHAT_CHANNELS: usize = 500; // TODO: Move to table

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "ChatChannelVisibility")]
#[repr(u8)]
pub enum ChatChannelVisibility {
    Unlisted,
    Controlled,
    Public,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "ChatChannelPermission")]
#[repr(i32)]
pub enum ChatChannelPermission {
    PendingInvitation,
    AccessRequested,
    Member,
    Officer,
    Banned, // Only Owner is stronger than Banned
    Owner,
}

#[spacetimedb::table(name = blocked_player_state, public,
    index(name = owner_entity_id, btree(columns = [owner_entity_id])),
    index(name = blocked_entity_id, btree(columns = [blocked_entity_id])),
    index(name = owner_blocked_entity_id, btree(columns = [owner_entity_id, blocked_entity_id]))
)]
#[derive(Clone, Debug)]
pub struct BlockedPlayerState {
    pub owner_entity_id: u64,
    pub blocked_entity_id: u64,
}

#[spacetimedb::table(name = friends_state, public,
    index(name = owner_entity_id, btree(columns = [owner_entity_id])),
    index(name = owner_friend_entity_id, btree(columns = [owner_entity_id, friend_entity_id]))
)]
#[derive(Clone, Debug)]
pub struct FriendsState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub friend_entity_id: u64,
    pub is_favorite: bool,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "VisibilityType")]
#[repr(i32)]
pub enum VisibilityType {
    Public,
    FriendsAndClaim,
    Friends,
    Private,
}

#[spacetimedb::table(name = visibility_state, public)]
#[derive(Clone, Debug)]
pub struct VisibilityState {
    #[primary_key]
    pub entity_id: u64,
    pub visibility: VisibilityType,
}

#[spacetimedb::table(name = user_creation_timestamp_state)]
#[derive(Clone, Debug)]
pub struct UserCreationTimestampState {
    #[primary_key]
    pub identity: Identity,
    pub timestamp: Timestamp,
}

#[spacetimedb::table(name = premium_purchase_state, public)]
#[derive(Clone, Debug)]
pub struct PremiumPurchaseState {
    #[primary_key]
    #[auto_inc]
    pub entity_id: u64,
    pub identity: Identity,
    pub collectible_desc_id: Option<i32>,
    pub price: u32,
    pub timestamp: Timestamp,
    pub processed: bool,
    #[default(1)]
    pub quantity: u32,
}
