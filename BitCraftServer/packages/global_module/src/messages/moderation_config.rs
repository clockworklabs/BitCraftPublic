// ============================================================================
// Moderation Enforcement Config Table
// ============================================================================

/// Boolean flags controlling which moderation features are active.
/// Singleton table (id = 0).
#[spacetimedb::table(accessor = mod_enforcement_config_state)]
#[derive(Clone, Debug)]
pub struct ModEnforcementConfigState {
    #[primary_key]
    pub id: u8, // always 0 (singleton)
    pub moderation_enforcement_active: bool,
    pub chat_moderation_enforcement_active: bool,
    pub username_moderation_enforcement_active: bool,
    pub entity_moderation_enforcement_active: bool,
    pub moderated_entity_name_types: u8, // EntityNameContextCode as flags
    pub check_for_links: bool,
    pub check_for_flagged_words: bool,
    pub check_for_context_flagged_words: bool,
    pub delete_flagged_messages: bool,
    pub allow_links_cwl: bool,
    pub title_id_cwl: i32,
    pub http_request_max_retries: i32,
}

// ============================================================================
// Moderation Threshold Table (individual entries with type)
// ============================================================================

/// Individual moderation threshold entries, filterable by type.
/// Suggested threshold_type values:
///   0 = Category, 1 = ModifyReplace, 2 = Global, 3 = EntityName
#[spacetimedb::table(accessor = mod_threshold_state,
    index(accessor = threshold_type, btree(columns = [threshold_type])))]
#[derive(Clone, Debug)]
pub struct ModThresholdState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub threshold_type: u8,
    pub category: String,
    pub threshold: f64,
}

// ============================================================================
// Flagged Word Table (individual entries with type)
// ============================================================================

/// Individual flagged word entries, filterable by type.
/// Suggested word_type values:
///   0 = Flagged, 1 = ContextFlagged, 2 = EntityName, 3 = KnownTld, 4 = WhitelistedUrl
///   5 = WhitelistUsername, 6 = WhitelistBuildingName, 7 = WhitelistClaimName
///   8 = WhitelistEmpireName, 9 = WhitelistEmpireRankName, 10 = WhitelistSignPost
#[spacetimedb::table(accessor = mod_flagged_word_state,
    index(accessor = word_type, btree(columns = [word_type])))]
#[derive(Clone, Debug)]
pub struct ModFlaggedWordState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub word_type: u8,
    pub word: String,
}

// ============================================================================
// Word Replacement Table (individual entries)
// ============================================================================

/// Individual word replacement entries for text preparation.
/// When is_developer_url_replacement is true, this row represents the
/// DeveloperURLWordReplacement (singleton); otherwise it is a SafeWordReplacement.
#[spacetimedb::table(accessor = mod_word_replacement_state)]
#[derive(Clone, Debug)]
pub struct ModWordReplacementState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub words_to_replace: Vec<String>,
    pub text_replacement: String,
    pub is_developer_url_replacement: bool,
}

// ============================================================================
// Replacement Text Table (individual entries with type)
// ============================================================================

/// Individual replacement text entries, keyed by type.
/// Suggested text_type values:
///   0 = FlaggedMessage, 1 = FlaggedMessageLinks
#[spacetimedb::table(accessor = mod_replacement_text_state)]
#[derive(Clone, Debug)]
pub struct ModReplacementTextState {
    #[primary_key]
    pub text_type: u8,
    pub text: String,
}

// ============================================================================
// Report Moderation Config Table (scalar settings only)
// ============================================================================

/// Report handling scalar configuration.
/// Singleton table (id = 0).
#[spacetimedb::table(accessor = mod_report_config_state)]
#[derive(Clone, Debug)]
pub struct ModReportConfigState {
    #[primary_key]
    pub id: u8,
    pub model: String,
    pub model_double_check: String,
    pub model_translate: String,
    pub offense_count_window_minutes: f32,
    pub min_minutes_between_offenses: f32,
    pub reportable_message_max_age: i32, // minutes
    pub count_admin_moderation_actions: bool,
    pub discord_webhook_url_user_reports: String,
}

// ============================================================================
// Consequence Table (individual entries)
// ============================================================================

/// Individual consequence entries for report moderation.
#[spacetimedb::table(accessor = mod_consequence_state)]
#[derive(Clone, Debug)]
pub struct ModConsequenceState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub point_threshold: i32,
    pub consequence_type: u8,
    pub duration: i32,
    pub flag_level_code: u8,
}

// ============================================================================
// Violation Table (individual entries)
// ============================================================================

/// Individual violation type entries for report moderation.
#[spacetimedb::table(accessor = mod_violation_state)]
#[derive(Clone, Debug)]
pub struct ModViolationState {
    #[primary_key]
    pub violation_type: u8,
    pub point_value_min: i32,
    pub point_value_max: i32,
}

// ============================================================================
// Flag Level Threshold Table (individual entries)
// ============================================================================

/// Individual flag level threshold entries for report moderation.
#[spacetimedb::table(accessor = mod_flag_level_threshold_state)]
#[derive(Clone, Debug)]
pub struct ModFlagLevelThresholdState {
    #[primary_key]
    pub flag_level: u8,
    pub point_threshold: i32,
}

// ============================================================================
// Playtime Threshold Table (individual entries)
// ============================================================================

/// Online time thresholds the CS portal uses to flag likely botting.
#[spacetimedb::table(accessor = mod_playtime_threshold_state)]
#[derive(Clone, Debug)]
pub struct ModPlaytimeThresholdState {
    #[primary_key]
    pub metric: u8,
    pub threshold_hours: i32,
    pub alert_enabled: bool,
}
