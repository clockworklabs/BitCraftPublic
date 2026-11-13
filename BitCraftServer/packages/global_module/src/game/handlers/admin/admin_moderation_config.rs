use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        moderation_config::{
            mod_consequence_state, mod_enforcement_config_state, mod_flag_level_threshold_state, mod_flagged_word_state,
            mod_playtime_threshold_state, mod_replacement_text_state, mod_report_config_state, mod_threshold_state, mod_violation_state,
            mod_word_replacement_state, ModConsequenceState, ModEnforcementConfigState, ModFlagLevelThresholdState, ModFlaggedWordState,
            ModPlaytimeThresholdState, ModReplacementTextState, ModReportConfigState, ModThresholdState, ModViolationState,
            ModWordReplacementState,
        },
    },
};

// ============================================================================
// Moderation Enforcement Config
// ============================================================================

#[spacetimedb::reducer]
#[expect(clippy::too_many_arguments, reason = "reducer arguments are part of the client API")]
pub fn admin_update_moderation_enforcement_config(
    ctx: &ReducerContext,
    moderation_enforcement_active: bool,
    chat_moderation_enforcement_active: bool,
    username_moderation_enforcement_active: bool,
    entity_moderation_enforcement_active: bool,
    moderated_entity_name_types: u8,
    check_for_links: bool,
    check_for_flagged_words: bool,
    check_for_context_flagged_words: bool,
    delete_flagged_messages: bool,
    allow_links_cwl: bool,
    title_id_cwl: i32,
    http_request_max_retries: i32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // Upsert pattern: delete existing, insert new
    ctx.db.mod_enforcement_config_state().id().delete(0);
    ctx.db.mod_enforcement_config_state().insert(ModEnforcementConfigState {
        id: 0,
        moderation_enforcement_active,
        chat_moderation_enforcement_active,
        username_moderation_enforcement_active,
        entity_moderation_enforcement_active,
        moderated_entity_name_types,
        check_for_links,
        check_for_flagged_words,
        check_for_context_flagged_words,
        delete_flagged_messages,
        allow_links_cwl,
        title_id_cwl,
        http_request_max_retries,
    });

    log::info!("[Admin] Updated moderation enforcement config");
    Ok(())
}

// ============================================================================
// Moderation Threshold Management
// ============================================================================

/// Add or update a moderation threshold entry. id == 0 means insert; id != 0 means update existing.
/// Suggested threshold_type values: 0=Category, 1=ModifyReplace, 2=Global, 3=EntityName
#[spacetimedb::reducer]
pub fn admin_add_or_update_moderation_threshold(
    ctx: &ReducerContext,
    id: u64,
    threshold_type: u8,
    category: String,
    threshold: f64,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if id == 0 {
        for row in ctx.db.mod_threshold_state().threshold_type().filter(&threshold_type) {
            if row.category == category {
                return Err("A threshold with this category already exists for this type. Use edit to update.".into());
            }
        }
        ctx.db.mod_threshold_state().insert(ModThresholdState {
            id: 0, // auto_inc
            threshold_type,
            category,
            threshold,
        });
        log::info!("[Admin] Added moderation threshold");
    } else {
        let existing = ctx.db.mod_threshold_state().id().find(id).ok_or("Threshold not found")?;
        for row in ctx.db.mod_threshold_state().threshold_type().filter(&threshold_type) {
            if row.id != existing.id && row.category == category {
                return Err("A threshold with this category already exists for this type.".into());
            }
        }
        ctx.db.mod_threshold_state().id().update(ModThresholdState {
            id: existing.id,
            threshold_type,
            category,
            threshold,
        });
        log::info!("[Admin] Updated moderation threshold id={}", id);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_moderation_threshold(ctx: &ReducerContext, id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_threshold_state().id().delete(id) {
        log::info!("[Admin] Removed moderation threshold id={}", id);
        Ok(())
    } else {
        Err("Threshold not found".into())
    }
} // ============================================================================
  // Flagged Word Management
  // ============================================================================

/// Add or update a flagged word entry. id == 0 means insert; id != 0 means update existing.
/// Suggested word_type values: 0=Flagged, 1=ContextFlagged, 2=EntityName, 3=KnownTld
#[spacetimedb::reducer]
pub fn admin_add_or_update_flagged_word(ctx: &ReducerContext, id: u64, word_type: u8, word: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if id == 0 {
        for row in ctx.db.mod_flagged_word_state().word_type().filter(&word_type) {
            if row.word == word {
                return Err("This word already exists for this type. Use edit to update.".into());
            }
        }
        ctx.db.mod_flagged_word_state().insert(ModFlaggedWordState {
            id: 0, // auto_inc
            word_type,
            word,
        });
        log::info!("[Admin] Added flagged word");
    } else {
        let existing = ctx.db.mod_flagged_word_state().id().find(id).ok_or("Flagged word not found")?;
        for row in ctx.db.mod_flagged_word_state().word_type().filter(&word_type) {
            if row.id != existing.id && row.word == word {
                return Err("This word already exists for this type.".into());
            }
        }
        ctx.db.mod_flagged_word_state().id().update(ModFlaggedWordState {
            id: existing.id,
            word_type,
            word,
        });
        log::info!("[Admin] Updated flagged word id={}", id);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_flagged_word(ctx: &ReducerContext, id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_flagged_word_state().id().delete(id) {
        log::info!("[Admin] Removed flagged word id={}", id);
        Ok(())
    } else {
        Err("Flagged word not found".into())
    }
} // ============================================================================
  // Word Replacement Management
  // ============================================================================

/// Add or update a word replacement. id == 0 means insert; id != 0 means update existing.
#[spacetimedb::reducer]
pub fn admin_add_or_update_word_replacement(
    ctx: &ReducerContext,
    id: u64,
    words_to_replace: Vec<String>,
    text_replacement: String,
    is_developer_url_replacement: bool,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if id == 0 {
        ctx.db.mod_word_replacement_state().insert(ModWordReplacementState {
            id: 0, // auto_inc
            words_to_replace,
            text_replacement,
            is_developer_url_replacement,
        });
        log::info!("[Admin] Added word replacement (developer_url={})", is_developer_url_replacement);
    } else {
        let existing = ctx
            .db
            .mod_word_replacement_state()
            .id()
            .find(id)
            .ok_or("Word replacement not found")?;
        ctx.db.mod_word_replacement_state().id().update(ModWordReplacementState {
            id: existing.id,
            words_to_replace,
            text_replacement,
            is_developer_url_replacement,
        });
        log::info!("[Admin] Updated word replacement id={}", id);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_word_replacement(ctx: &ReducerContext, id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_word_replacement_state().id().delete(id) {
        log::info!("[Admin] Removed word replacement id={}", id);
        Ok(())
    } else {
        Err("Word replacement not found".into())
    }
}
// ============================================================================
// Replacement Text Management
// ============================================================================

/// Add or update a replacement text entry.
/// Suggested text_type values: 0=FlaggedMessage, 1=FlaggedMessageLinks
#[spacetimedb::reducer]
pub fn admin_add_or_update_replacement_text(ctx: &ReducerContext, text_type: u8, text: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // Upsert: delete existing if present, then insert
    ctx.db.mod_replacement_text_state().text_type().delete(text_type);
    ctx.db
        .mod_replacement_text_state()
        .insert(ModReplacementTextState { text_type, text });

    log::info!("[Admin] Add or update replacement text type={}", text_type);
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_replacement_text(ctx: &ReducerContext, text_type: u8) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_replacement_text_state().text_type().delete(text_type) {
        log::info!("[Admin] Removed replacement text type={}", text_type);
        Ok(())
    } else {
        Err("Replacement text not found".into())
    }
}

// ============================================================================
// Report Moderation Config (scalar settings)
// ============================================================================

#[spacetimedb::reducer]
#[expect(clippy::too_many_arguments, reason = "reducer arguments are part of the client API")]
pub fn admin_update_report_moderation_config(
    ctx: &ReducerContext,
    model: String,
    model_double_check: String,
    model_translate: String,
    offense_count_window_minutes: f32,
    min_minutes_between_offenses: f32,
    reportable_message_max_age: i32,
    count_admin_moderation_actions: bool,
    discord_webhook_url_user_reports: String,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    ctx.db.mod_report_config_state().id().delete(0);
    ctx.db.mod_report_config_state().insert(ModReportConfigState {
        id: 0,
        model,
        model_double_check,
        model_translate,
        offense_count_window_minutes,
        min_minutes_between_offenses,
        reportable_message_max_age,
        count_admin_moderation_actions,
        discord_webhook_url_user_reports,
    });

    log::info!("[Admin] Updated report moderation config");
    Ok(())
}

// ============================================================================
// Consequence Management
// ============================================================================

/// Add or update a moderation consequence. id == 0 means insert; id != 0 means update existing.
#[spacetimedb::reducer]
pub fn admin_add_or_update_moderation_consequence(
    ctx: &ReducerContext,
    id: u64,
    point_threshold: i32,
    consequence_type: u8,
    duration: i32,
    flag_level_code: u8,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if id == 0 {
        ctx.db.mod_consequence_state().insert(ModConsequenceState {
            id: 0, // auto_inc
            point_threshold,
            consequence_type,
            duration,
            flag_level_code,
        });
        log::info!("[Admin] Added moderation consequence");
    } else {
        let existing = ctx.db.mod_consequence_state().id().find(id).ok_or("Consequence not found")?;
        ctx.db.mod_consequence_state().id().update(ModConsequenceState {
            id: existing.id,
            point_threshold,
            consequence_type,
            duration,
            flag_level_code,
        });
        log::info!("[Admin] Updated moderation consequence id={}", id);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_moderation_consequence(ctx: &ReducerContext, id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_consequence_state().id().delete(id) {
        log::info!("[Admin] Removed moderation consequence id={}", id);
        Ok(())
    } else {
        Err("Consequence not found".into())
    }
}
// ============================================================================
// Violation Management
// ============================================================================

#[spacetimedb::reducer]
pub fn admin_add_or_update_moderation_violation(
    ctx: &ReducerContext,
    violation_type: u8,
    point_value_min: i32,
    point_value_max: i32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // Upsert: delete existing if present, then insert
    ctx.db.mod_violation_state().violation_type().delete(violation_type);
    ctx.db.mod_violation_state().insert(ModViolationState {
        violation_type,
        point_value_min,
        point_value_max,
    });

    log::info!("[Admin] Add or update moderation violation type={}", violation_type);
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_moderation_violation(ctx: &ReducerContext, violation_type: u8) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_violation_state().violation_type().delete(violation_type) {
        log::info!("[Admin] Removed moderation violation type={}", violation_type);
        Ok(())
    } else {
        Err("Violation not found".into())
    }
}
// ============================================================================
// Flag Level Threshold Management
// ============================================================================

#[spacetimedb::reducer]
pub fn admin_add_or_update_moderation_flag_level_threshold(
    ctx: &ReducerContext,
    flag_level: u8,
    point_threshold: i32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // Upsert: delete existing if present, then insert
    ctx.db.mod_flag_level_threshold_state().flag_level().delete(flag_level);
    ctx.db.mod_flag_level_threshold_state().insert(ModFlagLevelThresholdState {
        flag_level,
        point_threshold,
    });

    log::info!("[Admin] Add or update moderation flag level threshold level={}", flag_level);
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_remove_moderation_flag_level_threshold(ctx: &ReducerContext, flag_level: u8) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if ctx.db.mod_flag_level_threshold_state().flag_level().delete(flag_level) {
        log::info!("[Admin] Removed moderation flag level threshold level={}", flag_level);
        Ok(())
    } else {
        Err("Flag level threshold not found".into())
    }
}

// ============================================================================
// Playtime Threshold Management
// ============================================================================

#[spacetimedb::reducer]
pub fn admin_update_playtime_threshold(ctx: &ReducerContext, metric: u8, threshold_hours: i32, alert_enabled: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if metric > 3 {
        return Err(format!("Invalid playtime metric {}", metric));
    }
    if threshold_hours <= 0 {
        return Err("Threshold must be at least 1 hour".into());
    }

    // Upsert: delete existing if present, then insert
    ctx.db.mod_playtime_threshold_state().metric().delete(metric);
    ctx.db.mod_playtime_threshold_state().insert(ModPlaytimeThresholdState {
        metric,
        threshold_hours,
        alert_enabled,
    });

    log::info!(
        "[Admin] Updated playtime threshold metric={} hours={} alert={}",
        metric,
        threshold_hours,
        alert_enabled
    );
    Ok(())
}
