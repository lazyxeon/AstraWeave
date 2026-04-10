//! Wave 2 Mutation Remediation — Import Doctor + Asset Browser per-variant tests
//!
//! Targets import_doctor_panel.rs + asset_browser.rs
//! - SourceEngine (14 variants × from_filename/uses_directx_normals/uses_opengl_normals/default_packing)
//! - TexturePackingFormat (6 variants × ao_channel/roughness_channel/metallic_channel/from_filename)
//! - IssueSeverity (4 variants × color/blocks_import/PartialOrd)
//! - IssueType (14 variants × default_severity/has_auto_fix/fix_description/suggested_fix)
//! - QuickFix (12 variants × is_destructive)
//! - ImportIssue builder
//! - TextureType (10 variants × from_filename 20+ suffixes, is_pbr_component, is_packed)
//! - AssetCategory matches, AssetAction, AssetType from_path, ViewMode

use aw_editor_lib::panels::asset_browser::{
    AssetAction, AssetCategory, AssetType, TextureType, ViewMode,
};
use aw_editor_lib::panels::import_doctor_panel::{
    ImportDoctorPanel, ImportIssue, ImportSettings, IssueSeverity, IssueType, QuickFix,
    SourceEngine, TexturePackingFormat, UpAxis,
};
use std::path::{Path, PathBuf};

// ============================================================================
// SOURCE ENGINE — FROM_FILENAME (pattern matching)
// ============================================================================

#[test]
fn source_engine_from_unreal() {
    assert_eq!(
        SourceEngine::from_filename("rock_unreal.fbx"),
        SourceEngine::Unreal
    );
}

#[test]
fn source_engine_from_ue4() {
    assert_eq!(
        SourceEngine::from_filename("mesh_ue4.obj"),
        SourceEngine::Unreal
    );
}

#[test]
fn source_engine_from_ue5() {
    assert_eq!(
        SourceEngine::from_filename("asset_ue5_high.fbx"),
        SourceEngine::Unreal
    );
}

#[test]
fn source_engine_from_unity() {
    assert_eq!(
        SourceEngine::from_filename("tree_unity.fbx"),
        SourceEngine::Unity
    );
}

#[test]
fn source_engine_from_blender() {
    assert_eq!(
        SourceEngine::from_filename("house_blender.glb"),
        SourceEngine::Blender
    );
}

#[test]
fn source_engine_from_blend() {
    assert_eq!(
        SourceEngine::from_filename("char_blend.fbx"),
        SourceEngine::Blender
    );
}

#[test]
fn source_engine_from_substance_painter() {
    assert_eq!(
        SourceEngine::from_filename("mat_sp_export.png"),
        SourceEngine::SubstancePainter
    );
}

#[test]
fn source_engine_from_substance() {
    assert_eq!(
        SourceEngine::from_filename("tex_substance.png"),
        SourceEngine::SubstancePainter
    );
}

#[test]
fn source_engine_from_quixel() {
    assert_eq!(
        SourceEngine::from_filename("rock_quixel_8k.jpg"),
        SourceEngine::Quixel
    );
}

#[test]
fn source_engine_from_megascans() {
    assert_eq!(
        SourceEngine::from_filename("ground_megascans_2k.png"),
        SourceEngine::Quixel
    );
}

#[test]
fn source_engine_from_maya() {
    assert_eq!(
        SourceEngine::from_filename("char_maya.fbx"),
        SourceEngine::Maya
    );
}

#[test]
fn source_engine_from_3dsmax() {
    assert_eq!(
        SourceEngine::from_filename("building_3dsmax.obj"),
        SourceEngine::ThreeDSMax
    );
}

#[test]
fn source_engine_from_max() {
    assert_eq!(
        SourceEngine::from_filename("prop_max.fbx"),
        SourceEngine::ThreeDSMax
    );
}

#[test]
fn source_engine_from_cinema4d() {
    assert_eq!(
        SourceEngine::from_filename("logo_cinema4d.obj"),
        SourceEngine::Cinema4D
    );
}

#[test]
fn source_engine_from_c4d() {
    assert_eq!(
        SourceEngine::from_filename("widget_c4d.fbx"),
        SourceEngine::Cinema4D
    );
}

#[test]
fn source_engine_from_houdini() {
    assert_eq!(
        SourceEngine::from_filename("terrain_houdini.obj"),
        SourceEngine::Houdini
    );
}

#[test]
fn source_engine_from_zbrush() {
    assert_eq!(
        SourceEngine::from_filename("sculpt_zbrush.obj"),
        SourceEngine::ZBrush
    );
}

#[test]
fn source_engine_from_unknown() {
    assert_eq!(
        SourceEngine::from_filename("random_mesh.glb"),
        SourceEngine::Unknown
    );
}

// ============================================================================
// SOURCE ENGINE — NORMAL MAP CONVENTIONS
// ============================================================================

#[test]
fn source_engine_unity_uses_directx() {
    assert!(SourceEngine::Unity.uses_directx_normals());
}

#[test]
fn source_engine_unreal_uses_directx() {
    assert!(SourceEngine::Unreal.uses_directx_normals());
}

#[test]
fn source_engine_3dsmax_uses_directx() {
    assert!(SourceEngine::ThreeDSMax.uses_directx_normals());
}

#[test]
fn source_engine_quixel_uses_directx() {
    assert!(SourceEngine::Quixel.uses_directx_normals());
}

#[test]
fn source_engine_blender_not_directx() {
    assert!(!SourceEngine::Blender.uses_directx_normals());
}

#[test]
fn source_engine_blender_uses_opengl() {
    assert!(SourceEngine::Blender.uses_opengl_normals());
}

#[test]
fn source_engine_maya_uses_opengl() {
    assert!(SourceEngine::Maya.uses_opengl_normals());
}

#[test]
fn source_engine_substance_painter_uses_opengl() {
    assert!(SourceEngine::SubstancePainter.uses_opengl_normals());
}

#[test]
fn source_engine_substance_designer_uses_opengl() {
    assert!(SourceEngine::SubstanceDesigner.uses_opengl_normals());
}

#[test]
fn source_engine_cinema4d_uses_opengl() {
    assert!(SourceEngine::Cinema4D.uses_opengl_normals());
}

#[test]
fn source_engine_houdini_uses_opengl() {
    assert!(SourceEngine::Houdini.uses_opengl_normals());
}

#[test]
fn source_engine_unknown_neither() {
    assert!(!SourceEngine::Unknown.uses_directx_normals());
    assert!(!SourceEngine::Unknown.uses_opengl_normals());
}

#[test]
fn source_engine_custom_neither() {
    assert!(!SourceEngine::Custom.uses_directx_normals());
    assert!(!SourceEngine::Custom.uses_opengl_normals());
}

#[test]
fn source_engine_photoshop_neither() {
    assert!(!SourceEngine::Photoshop.uses_directx_normals());
    assert!(!SourceEngine::Photoshop.uses_opengl_normals());
}

#[test]
fn source_engine_zbrush_neither() {
    assert!(!SourceEngine::ZBrush.uses_directx_normals());
    assert!(!SourceEngine::ZBrush.uses_opengl_normals());
}

// ============================================================================
// SOURCE ENGINE — DEFAULT PACKING
// ============================================================================

#[test]
fn source_engine_unreal_default_packing_orm() {
    assert_eq!(
        SourceEngine::Unreal.default_packing(),
        TexturePackingFormat::ORM
    );
}

#[test]
fn source_engine_unity_default_packing_mra() {
    assert_eq!(
        SourceEngine::Unity.default_packing(),
        TexturePackingFormat::MRA
    );
}

#[test]
fn source_engine_substance_painter_default_packing_orm() {
    assert_eq!(
        SourceEngine::SubstancePainter.default_packing(),
        TexturePackingFormat::ORM
    );
}

#[test]
fn source_engine_substance_designer_default_packing_orm() {
    assert_eq!(
        SourceEngine::SubstanceDesigner.default_packing(),
        TexturePackingFormat::ORM
    );
}

#[test]
fn source_engine_quixel_default_packing_orm() {
    assert_eq!(
        SourceEngine::Quixel.default_packing(),
        TexturePackingFormat::ORM
    );
}

#[test]
fn source_engine_unknown_default_packing_separate() {
    assert_eq!(
        SourceEngine::Unknown.default_packing(),
        TexturePackingFormat::Separate
    );
}

#[test]
fn source_engine_blender_default_packing_separate() {
    assert_eq!(
        SourceEngine::Blender.default_packing(),
        TexturePackingFormat::Separate
    );
}

#[test]
fn source_engine_all_count() {
    assert_eq!(SourceEngine::all().len(), 14);
}

#[test]
fn source_engine_names_all_nonempty() {
    for se in SourceEngine::all() {
        assert!(!se.name().is_empty());
        assert!(!se.icon().is_empty());
    }
}

#[test]
fn source_engine_display_contains_name() {
    for se in SourceEngine::all() {
        let s = format!("{}", se);
        assert!(s.contains(se.name()));
    }
}

// ============================================================================
// TEXTURE PACKING FORMAT — CHANNEL MAPPING
// ============================================================================

#[test]
fn packing_separate_no_channels() {
    assert_eq!(TexturePackingFormat::Separate.ao_channel(), None);
    assert_eq!(TexturePackingFormat::Separate.roughness_channel(), None);
    assert_eq!(TexturePackingFormat::Separate.metallic_channel(), None);
}

#[test]
fn packing_orm_channels() {
    assert_eq!(TexturePackingFormat::ORM.ao_channel(), Some('R'));
    assert_eq!(TexturePackingFormat::ORM.roughness_channel(), Some('G'));
    assert_eq!(TexturePackingFormat::ORM.metallic_channel(), Some('B'));
}

#[test]
fn packing_mra_channels() {
    assert_eq!(TexturePackingFormat::MRA.ao_channel(), Some('B'));
    assert_eq!(TexturePackingFormat::MRA.roughness_channel(), Some('G'));
    assert_eq!(TexturePackingFormat::MRA.metallic_channel(), Some('R'));
}

#[test]
fn packing_rma_channels() {
    assert_eq!(TexturePackingFormat::RMA.ao_channel(), Some('B'));
    assert_eq!(TexturePackingFormat::RMA.roughness_channel(), Some('R'));
    assert_eq!(TexturePackingFormat::RMA.metallic_channel(), Some('G'));
}

#[test]
fn packing_arm_channels() {
    assert_eq!(TexturePackingFormat::ARM.ao_channel(), Some('R'));
    assert_eq!(TexturePackingFormat::ARM.roughness_channel(), Some('G'));
    assert_eq!(TexturePackingFormat::ARM.metallic_channel(), Some('B'));
}

#[test]
fn packing_mro_channels() {
    assert_eq!(TexturePackingFormat::MRO.ao_channel(), Some('B'));
    assert_eq!(TexturePackingFormat::MRO.roughness_channel(), Some('G'));
    assert_eq!(TexturePackingFormat::MRO.metallic_channel(), Some('R'));
}

#[test]
fn packing_from_filename_orm() {
    assert_eq!(
        TexturePackingFormat::from_filename("rock_orm.png"),
        TexturePackingFormat::ORM
    );
}

#[test]
fn packing_from_filename_mra() {
    assert_eq!(
        TexturePackingFormat::from_filename("rock_mra.png"),
        TexturePackingFormat::MRA
    );
}

#[test]
fn packing_from_filename_rma() {
    assert_eq!(
        TexturePackingFormat::from_filename("rock_rma.png"),
        TexturePackingFormat::RMA
    );
}

#[test]
fn packing_from_filename_arm() {
    assert_eq!(
        TexturePackingFormat::from_filename("rock_arm.png"),
        TexturePackingFormat::ARM
    );
}

#[test]
fn packing_from_filename_mro() {
    assert_eq!(
        TexturePackingFormat::from_filename("rock_mro.png"),
        TexturePackingFormat::MRO
    );
}

#[test]
fn packing_from_filename_unknown() {
    assert_eq!(
        TexturePackingFormat::from_filename("rock_albedo.png"),
        TexturePackingFormat::Separate
    );
}

#[test]
fn packing_all_count() {
    assert_eq!(TexturePackingFormat::all().len(), 6);
}

#[test]
fn packing_names() {
    assert_eq!(TexturePackingFormat::Separate.name(), "Separate");
    assert_eq!(TexturePackingFormat::ORM.name(), "ORM");
    assert_eq!(TexturePackingFormat::MRA.name(), "MRA");
    assert_eq!(TexturePackingFormat::RMA.name(), "RMA");
    assert_eq!(TexturePackingFormat::ARM.name(), "ARM");
    assert_eq!(TexturePackingFormat::MRO.name(), "MRO");
}

#[test]
fn packing_descriptions_nonempty() {
    for p in TexturePackingFormat::all() {
        assert!(!p.description().is_empty());
    }
}

// ============================================================================
// ISSUE SEVERITY — ORDERING / BLOCKS_IMPORT
// ============================================================================

#[test]
fn severity_info_does_not_block() {
    assert!(!IssueSeverity::Info.blocks_import());
}

#[test]
fn severity_warning_does_not_block() {
    assert!(!IssueSeverity::Warning.blocks_import());
}

#[test]
fn severity_error_does_not_block() {
    assert!(!IssueSeverity::Error.blocks_import());
}

#[test]
fn severity_critical_blocks_import() {
    assert!(IssueSeverity::Critical.blocks_import());
}

#[test]
fn severity_ordering() {
    assert!(IssueSeverity::Info < IssueSeverity::Warning);
    assert!(IssueSeverity::Warning < IssueSeverity::Error);
    assert!(IssueSeverity::Error < IssueSeverity::Critical);
}

#[test]
fn severity_all_count() {
    assert_eq!(IssueSeverity::all().len(), 4);
}

#[test]
fn severity_names() {
    assert_eq!(IssueSeverity::Info.name(), "Info");
    assert_eq!(IssueSeverity::Warning.name(), "Warning");
    assert_eq!(IssueSeverity::Error.name(), "Error");
    assert_eq!(IssueSeverity::Critical.name(), "Critical");
}

#[test]
fn severity_default_is_info() {
    assert_eq!(IssueSeverity::default(), IssueSeverity::Info);
}

// ============================================================================
// ISSUE TYPE — DEFAULT SEVERITY
// ============================================================================

#[test]
fn issue_type_unknown_severity() {
    assert_eq!(
        IssueType::Unknown.default_severity(),
        IssueSeverity::Warning
    );
}

#[test]
fn issue_type_normal_map_severity() {
    assert_eq!(
        IssueType::NormalMapFormat.default_severity(),
        IssueSeverity::Warning
    );
}

#[test]
fn issue_type_texture_packing_severity() {
    assert_eq!(
        IssueType::TexturePacking.default_severity(),
        IssueSeverity::Info
    );
}

#[test]
fn issue_type_missing_uvs_severity() {
    assert_eq!(
        IssueType::MissingUVs.default_severity(),
        IssueSeverity::Error
    );
}

#[test]
fn issue_type_unsupported_format_severity() {
    assert_eq!(
        IssueType::UnsupportedFormat.default_severity(),
        IssueSeverity::Critical
    );
}

#[test]
fn issue_type_missing_texture_severity() {
    assert_eq!(
        IssueType::MissingTexture.default_severity(),
        IssueSeverity::Error
    );
}

#[test]
fn issue_type_missing_collider_severity() {
    assert_eq!(
        IssueType::MissingCollider.default_severity(),
        IssueSeverity::Info
    );
}

#[test]
fn issue_type_missing_lods_severity() {
    assert_eq!(
        IssueType::MissingLODs.default_severity(),
        IssueSeverity::Info
    );
}

#[test]
fn issue_type_duplicate_material_severity() {
    assert_eq!(
        IssueType::DuplicateMaterial.default_severity(),
        IssueSeverity::Info
    );
}

// ============================================================================
// ISSUE TYPE — HAS_AUTO_FIX
// ============================================================================

#[test]
fn issue_type_normal_map_has_fix() {
    assert!(IssueType::NormalMapFormat.has_auto_fix());
}

#[test]
fn issue_type_texture_packing_has_fix() {
    assert!(IssueType::TexturePacking.has_auto_fix());
}

#[test]
fn issue_type_missing_tangents_has_fix() {
    assert!(IssueType::MissingTangents.has_auto_fix());
}

#[test]
fn issue_type_non_pot_has_fix() {
    assert!(IssueType::NonPowerOfTwo.has_auto_fix());
}

#[test]
fn issue_type_incorrect_scale_has_fix() {
    assert!(IssueType::IncorrectScale.has_auto_fix());
}

#[test]
fn issue_type_incorrect_orientation_has_fix() {
    assert!(IssueType::IncorrectOrientation.has_auto_fix());
}

#[test]
fn issue_type_missing_uvs_no_fix() {
    assert!(!IssueType::MissingUVs.has_auto_fix());
}

#[test]
fn issue_type_unsupported_format_no_fix() {
    assert!(!IssueType::UnsupportedFormat.has_auto_fix());
}

#[test]
fn issue_type_missing_texture_no_fix() {
    assert!(!IssueType::MissingTexture.has_auto_fix());
}

#[test]
fn issue_type_duplicate_material_no_fix() {
    assert!(!IssueType::DuplicateMaterial.has_auto_fix());
}

// ============================================================================
// ISSUE TYPE — FIX_DESCRIPTION
// ============================================================================

#[test]
fn issue_type_normal_map_fix_description() {
    assert!(IssueType::NormalMapFormat.fix_description().is_some());
}

#[test]
fn issue_type_texture_packing_fix_description() {
    assert!(IssueType::TexturePacking.fix_description().is_some());
}

#[test]
fn issue_type_missing_tangents_fix_description() {
    assert!(IssueType::MissingTangents.fix_description().is_some());
}

#[test]
fn issue_type_non_pot_fix_description() {
    assert!(IssueType::NonPowerOfTwo.fix_description().is_some());
}

#[test]
fn issue_type_missing_uvs_no_fix_description() {
    assert!(IssueType::MissingUVs.fix_description().is_none());
}

#[test]
fn issue_type_oversized_no_fix_description() {
    assert!(IssueType::OversizedTexture.fix_description().is_none());
}

// ============================================================================
// ISSUE TYPE — SUGGESTED FIX
// ============================================================================

#[test]
fn issue_type_normal_map_suggested_fix() {
    assert_eq!(
        IssueType::NormalMapFormat.suggested_fix(),
        Some(QuickFix::FlipGreenChannel)
    );
}

#[test]
fn issue_type_texture_packing_suggested_fix() {
    assert_eq!(
        IssueType::TexturePacking.suggested_fix(),
        Some(QuickFix::ConvertToORM)
    );
}

#[test]
fn issue_type_missing_tangents_suggested_fix() {
    assert_eq!(
        IssueType::MissingTangents.suggested_fix(),
        Some(QuickFix::GenerateTangents)
    );
}

#[test]
fn issue_type_non_pot_suggested_fix() {
    assert_eq!(
        IssueType::NonPowerOfTwo.suggested_fix(),
        Some(QuickFix::ResizePowerOfTwo)
    );
}

#[test]
fn issue_type_incorrect_scale_suggested_fix() {
    assert_eq!(
        IssueType::IncorrectScale.suggested_fix(),
        Some(QuickFix::FixScale)
    );
}

#[test]
fn issue_type_incorrect_orientation_suggested_fix() {
    assert_eq!(
        IssueType::IncorrectOrientation.suggested_fix(),
        Some(QuickFix::FixOrientation)
    );
}

#[test]
fn issue_type_missing_lods_suggested_fix() {
    assert_eq!(
        IssueType::MissingLODs.suggested_fix(),
        Some(QuickFix::GenerateLODs)
    );
}

#[test]
fn issue_type_missing_collider_suggested_fix() {
    assert_eq!(
        IssueType::MissingCollider.suggested_fix(),
        Some(QuickFix::GenerateCollider)
    );
}

#[test]
fn issue_type_missing_uvs_no_suggested_fix() {
    assert!(IssueType::MissingUVs.suggested_fix().is_none());
}

#[test]
fn issue_type_unsupported_format_no_suggested_fix() {
    assert!(IssueType::UnsupportedFormat.suggested_fix().is_none());
}

#[test]
fn issue_type_all_count() {
    assert_eq!(IssueType::all().len(), 14);
}

#[test]
fn issue_type_names_nonempty() {
    for it in IssueType::all() {
        assert!(!it.name().is_empty());
        assert!(!it.icon().is_empty());
    }
}

// ============================================================================
// QUICK FIX — IS_DESTRUCTIVE
// ============================================================================

#[test]
fn quickfix_flip_green_destructive() {
    assert!(QuickFix::FlipGreenChannel.is_destructive());
}

#[test]
fn quickfix_convert_orm_destructive() {
    assert!(QuickFix::ConvertToORM.is_destructive());
}

#[test]
fn quickfix_convert_mra_destructive() {
    assert!(QuickFix::ConvertToMRA.is_destructive());
}

#[test]
fn quickfix_resize_pot_destructive() {
    assert!(QuickFix::ResizePowerOfTwo.is_destructive());
}

#[test]
fn quickfix_treat_as_normal_not_destructive() {
    assert!(!QuickFix::TreatAsNormalMap.is_destructive());
}

#[test]
fn quickfix_generate_tangents_not_destructive() {
    assert!(!QuickFix::GenerateTangents.is_destructive());
}

#[test]
fn quickfix_generate_lods_not_destructive() {
    assert!(!QuickFix::GenerateLODs.is_destructive());
}

#[test]
fn quickfix_generate_collider_not_destructive() {
    assert!(!QuickFix::GenerateCollider.is_destructive());
}

#[test]
fn quickfix_fix_scale_not_destructive() {
    assert!(!QuickFix::FixScale.is_destructive());
}

#[test]
fn quickfix_fix_orientation_not_destructive() {
    assert!(!QuickFix::FixOrientation.is_destructive());
}

#[test]
fn quickfix_mark_srgb_not_destructive() {
    assert!(!QuickFix::MarkAsSRGB.is_destructive());
}

#[test]
fn quickfix_mark_linear_not_destructive() {
    assert!(!QuickFix::MarkAsLinear.is_destructive());
}

#[test]
fn quickfix_all_count() {
    assert_eq!(QuickFix::all().len(), 12);
}

#[test]
fn quickfix_names_nonempty() {
    for qf in QuickFix::all() {
        assert!(!qf.name().is_empty());
        assert!(!qf.icon().is_empty());
        assert!(!qf.description().is_empty());
    }
}

// ============================================================================
// IMPORT ISSUE — BUILDER
// ============================================================================

#[test]
fn import_issue_new_defaults() {
    let issue = ImportIssue::new(IssueType::MissingTangents, "No tangents");
    assert_eq!(issue.issue_type, IssueType::MissingTangents);
    assert_eq!(issue.severity, IssueSeverity::Warning); // default for MissingTangents
    assert_eq!(issue.message, "No tangents");
    assert!(issue.file_path.is_none());
    assert!(issue.can_auto_fix); // MissingTangents has auto fix
    assert!(!issue.fix_applied);
}

#[test]
fn import_issue_with_file() {
    let issue = ImportIssue::new(IssueType::MissingUVs, "No UV channel")
        .with_file(PathBuf::from("mesh.fbx"));
    assert_eq!(issue.file_path, Some(PathBuf::from("mesh.fbx")));
}

#[test]
fn import_issue_with_severity_override() {
    let issue = ImportIssue::new(IssueType::MissingCollider, "No collider")
        .with_severity(IssueSeverity::Error);
    assert_eq!(issue.severity, IssueSeverity::Error); // overridden from Info
}

#[test]
fn import_issue_no_auto_fix_for_missing_uvs() {
    let issue = ImportIssue::new(IssueType::MissingUVs, "Missing UVs");
    assert!(!issue.can_auto_fix);
}

#[test]
fn import_issue_unsupported_format_is_critical() {
    let issue = ImportIssue::new(IssueType::UnsupportedFormat, "Bad format");
    assert_eq!(issue.severity, IssueSeverity::Critical);
}

// ============================================================================
// UP AXIS
// ============================================================================

#[test]
fn up_axis_y_name() {
    assert_eq!(UpAxis::Y.name(), "Y-Up");
}

#[test]
fn up_axis_z_name() {
    assert_eq!(UpAxis::Z.name(), "Z-Up");
}

#[test]
fn up_axis_default_is_y() {
    assert_eq!(UpAxis::default(), UpAxis::Y);
}

#[test]
fn up_axis_display() {
    assert_eq!(format!("{}", UpAxis::Y), "Y-Up");
    assert_eq!(format!("{}", UpAxis::Z), "Z-Up");
}

// ============================================================================
// IMPORT SETTINGS — DEFAULTS
// ============================================================================

#[test]
fn import_settings_defaults() {
    let s = ImportSettings::default();
    assert!(s.auto_detect_source);
    assert!(s.source_override.is_none());
    assert_eq!(s.packing_format, TexturePackingFormat::ORM);
    assert!(!s.flip_normal_green);
    assert!(s.generate_tangents);
    assert!(!s.resize_non_pot);
    assert!(s.generate_lods);
    assert_eq!(s.lod_levels, 3);
    assert!(!s.generate_colliders);
    assert!(s.fix_scale);
    assert!((s.scale_factor - 1.0).abs() < 0.001);
    assert!(s.fix_orientation);
    assert_eq!(s.target_up_axis, UpAxis::Y);
    assert!(s.show_preview);
}

// ============================================================================
// IMPORT DOCTOR PANEL — STATE
// ============================================================================

#[test]
fn panel_new_defaults() {
    let p = ImportDoctorPanel::new();
    assert_eq!(p.detected_source, SourceEngine::Unknown);
    assert_eq!(p.detected_packing, TexturePackingFormat::Separate);
    assert!(p.issues.is_empty());
    assert!(p.selected_files.is_empty());
    assert!(!p.preview_ready);
    assert!(!p.scanning);
    assert!((p.scan_progress - 0.0).abs() < 0.001);
    assert!(p.quick_fixes_applied.is_empty());
}

#[test]
fn panel_has_no_pending_actions() {
    let p = ImportDoctorPanel::new();
    assert!(!p.has_pending_actions());
}

#[test]
fn panel_take_actions_empty() {
    let mut p = ImportDoctorPanel::new();
    let actions = p.take_actions();
    assert!(actions.is_empty());
}

#[test]
fn panel_issue_count_empty() {
    let p = ImportDoctorPanel::new();
    assert_eq!(p.issue_count(IssueSeverity::Critical), 0);
    assert_eq!(p.issue_count(IssueSeverity::Error), 0);
}

#[test]
fn panel_fixable_count_empty() {
    let p = ImportDoctorPanel::new();
    assert_eq!(p.fixable_count(), 0);
}

#[test]
fn panel_can_import_no_issues() {
    let p = ImportDoctorPanel::new();
    assert!(p.can_import());
}

#[test]
fn panel_can_import_with_warning() {
    let mut p = ImportDoctorPanel::new();
    p.issues
        .push(ImportIssue::new(IssueType::NonPowerOfTwo, "Not POT"));
    assert!(p.can_import()); // Warning doesn't block
}

#[test]
fn panel_cannot_import_with_critical() {
    let mut p = ImportDoctorPanel::new();
    p.issues
        .push(ImportIssue::new(IssueType::UnsupportedFormat, "Bad format"));
    assert!(!p.can_import()); // Critical blocks
}

#[test]
fn panel_issue_count_by_severity() {
    let mut p = ImportDoctorPanel::new();
    p.issues
        .push(ImportIssue::new(IssueType::NonPowerOfTwo, "a"));
    p.issues
        .push(ImportIssue::new(IssueType::MissingTangents, "b"));
    p.issues.push(ImportIssue::new(IssueType::MissingUVs, "c"));
    assert_eq!(p.issue_count(IssueSeverity::Warning), 2); // NonPOT + MissingTangents
    assert_eq!(p.issue_count(IssueSeverity::Error), 1); // MissingUVs
    assert_eq!(p.issue_count(IssueSeverity::Critical), 0);
}

#[test]
fn panel_fixable_count_excludes_applied() {
    let mut p = ImportDoctorPanel::new();
    p.issues
        .push(ImportIssue::new(IssueType::MissingTangents, "a"));
    let mut applied = ImportIssue::new(IssueType::NonPowerOfTwo, "b");
    applied.fix_applied = true;
    p.issues.push(applied);
    assert_eq!(p.fixable_count(), 1); // Only first is fixable + not applied
}

// ============================================================================
// TEXTURE TYPE — FROM_FILENAME (20+ suffixes)
// ============================================================================

#[test]
fn texture_type_from_normal_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_normal.png"),
        TextureType::Normal
    );
}

#[test]
fn texture_type_from_n_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_n.png"),
        TextureType::Normal
    );
}

#[test]
fn texture_type_from_nrm_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_nrm.png"),
        TextureType::Normal
    );
}

#[test]
fn texture_type_from_nor_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_nor.png"),
        TextureType::Normal
    );
}

#[test]
fn texture_type_from_orm_suffix() {
    assert_eq!(TextureType::from_filename("rock_orm.png"), TextureType::ORM);
}

#[test]
fn texture_type_from_mra_suffix() {
    assert_eq!(TextureType::from_filename("rock_mra.png"), TextureType::MRA);
}

#[test]
fn texture_type_from_r_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_r.png"),
        TextureType::Roughness
    );
}

#[test]
fn texture_type_from_rough_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_rough.png"),
        TextureType::Roughness
    );
}

#[test]
fn texture_type_from_roughness_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_roughness.png"),
        TextureType::Roughness
    );
}

#[test]
fn texture_type_from_m_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_m.png"),
        TextureType::Metallic
    );
}

#[test]
fn texture_type_from_metal_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_metal.png"),
        TextureType::Metallic
    );
}

#[test]
fn texture_type_from_metallic_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_metallic.png"),
        TextureType::Metallic
    );
}

#[test]
fn texture_type_from_metalness_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_metalness.png"),
        TextureType::Metallic
    );
}

#[test]
fn texture_type_from_ao_suffix() {
    assert_eq!(TextureType::from_filename("rock_ao.png"), TextureType::AO);
}

#[test]
fn texture_type_from_occlusion_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_occlusion.png"),
        TextureType::AO
    );
}

#[test]
fn texture_type_from_e_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_e.png"),
        TextureType::Emission
    );
}

#[test]
fn texture_type_from_emit_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_emit.png"),
        TextureType::Emission
    );
}

#[test]
fn texture_type_from_emission_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_emission.png"),
        TextureType::Emission
    );
}

#[test]
fn texture_type_from_emissive_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_emissive.png"),
        TextureType::Emission
    );
}

#[test]
fn texture_type_from_glow_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_glow.png"),
        TextureType::Emission
    );
}

#[test]
fn texture_type_from_h_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_h.png"),
        TextureType::Height
    );
}

#[test]
fn texture_type_from_height_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_height.png"),
        TextureType::Height
    );
}

#[test]
fn texture_type_from_disp_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_disp.png"),
        TextureType::Height
    );
}

#[test]
fn texture_type_from_displacement_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_displacement.png"),
        TextureType::Height
    );
}

#[test]
fn texture_type_from_bump_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_bump.png"),
        TextureType::Height
    );
}

#[test]
fn texture_type_from_albedo_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_albedo.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_diffuse_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_diffuse.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_basecolor_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_basecolor.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_base_color_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_base_color.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_color_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_color.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_col_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_col.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_d_suffix() {
    assert_eq!(
        TextureType::from_filename("rock_d.png"),
        TextureType::Albedo
    );
}

#[test]
fn texture_type_from_unknown() {
    assert_eq!(
        TextureType::from_filename("random_file.png"),
        TextureType::Unknown
    );
}

// ============================================================================
// TEXTURE TYPE — IS_PBR_COMPONENT / IS_PACKED
// ============================================================================

#[test]
fn texture_type_unknown_not_pbr() {
    assert!(!TextureType::Unknown.is_pbr_component());
}

#[test]
fn texture_type_albedo_is_pbr() {
    assert!(TextureType::Albedo.is_pbr_component());
}

#[test]
fn texture_type_normal_is_pbr() {
    assert!(TextureType::Normal.is_pbr_component());
}

#[test]
fn texture_type_orm_is_packed() {
    assert!(TextureType::ORM.is_packed());
}

#[test]
fn texture_type_mra_is_packed() {
    assert!(TextureType::MRA.is_packed());
}

#[test]
fn texture_type_roughness_not_packed() {
    assert!(!TextureType::Roughness.is_packed());
}

#[test]
fn texture_type_albedo_not_packed() {
    assert!(!TextureType::Albedo.is_packed());
}

#[test]
fn texture_type_all_count() {
    assert_eq!(TextureType::all().len(), 10);
}

#[test]
fn texture_type_names_nonempty() {
    for t in TextureType::all() {
        assert!(!t.name().is_empty());
        assert!(!t.icon().is_empty());
        assert!(!t.label().is_empty());
    }
}

// ============================================================================
// ASSET CATEGORY — MATCHES
// ============================================================================

#[test]
fn category_all_matches_everything() {
    for at in AssetType::all() {
        assert!(AssetCategory::All.matches(at));
    }
}

#[test]
fn category_models_matches_model() {
    assert!(AssetCategory::Models.matches(&AssetType::Model));
    assert!(!AssetCategory::Models.matches(&AssetType::Texture));
}

#[test]
fn category_textures_matches_texture() {
    assert!(AssetCategory::Textures.matches(&AssetType::Texture));
    assert!(!AssetCategory::Textures.matches(&AssetType::Model));
}

#[test]
fn category_materials_matches_material() {
    assert!(AssetCategory::Materials.matches(&AssetType::Material));
}

#[test]
fn category_prefabs_matches_prefab() {
    assert!(AssetCategory::Prefabs.matches(&AssetType::Prefab));
}

#[test]
fn category_scenes_matches_scene() {
    assert!(AssetCategory::Scenes.matches(&AssetType::Scene));
}

#[test]
fn category_audio_matches_audio() {
    assert!(AssetCategory::Audio.matches(&AssetType::Audio));
}

#[test]
fn category_configs_matches_config() {
    assert!(AssetCategory::Configs.matches(&AssetType::Config));
}

#[test]
fn category_all_count() {
    assert_eq!(AssetCategory::all().len(), 8);
}

// ============================================================================
// ASSET TYPE — FROM_PATH
// ============================================================================

#[test]
fn asset_type_from_glb() {
    assert_eq!(
        AssetType::from_path(Path::new("mesh.glb")),
        AssetType::Model
    );
}

#[test]
fn asset_type_from_gltf() {
    assert_eq!(
        AssetType::from_path(Path::new("mesh.gltf")),
        AssetType::Model
    );
}

#[test]
fn asset_type_from_obj() {
    assert_eq!(
        AssetType::from_path(Path::new("mesh.obj")),
        AssetType::Model
    );
}

#[test]
fn asset_type_from_fbx() {
    assert_eq!(
        AssetType::from_path(Path::new("mesh.fbx")),
        AssetType::Model
    );
}

#[test]
fn asset_type_from_png() {
    assert_eq!(
        AssetType::from_path(Path::new("tex.png")),
        AssetType::Texture
    );
}

#[test]
fn asset_type_from_jpg() {
    assert_eq!(
        AssetType::from_path(Path::new("tex.jpg")),
        AssetType::Texture
    );
}

#[test]
fn asset_type_from_jpeg() {
    assert_eq!(
        AssetType::from_path(Path::new("tex.jpeg")),
        AssetType::Texture
    );
}

#[test]
fn asset_type_from_ktx2() {
    assert_eq!(
        AssetType::from_path(Path::new("tex.ktx2")),
        AssetType::Texture
    );
}

#[test]
fn asset_type_from_dds() {
    assert_eq!(
        AssetType::from_path(Path::new("tex.dds")),
        AssetType::Texture
    );
}

#[test]
fn asset_type_from_ron() {
    assert_eq!(
        AssetType::from_path(Path::new("scene.ron")),
        AssetType::Scene
    );
}

#[test]
fn asset_type_from_toml() {
    assert_eq!(
        AssetType::from_path(Path::new("config.toml")),
        AssetType::Config
    );
}

#[test]
fn asset_type_from_json() {
    assert_eq!(
        AssetType::from_path(Path::new("data.json")),
        AssetType::Config
    );
}

#[test]
fn asset_type_from_wav() {
    assert_eq!(
        AssetType::from_path(Path::new("sound.wav")),
        AssetType::Audio
    );
}

#[test]
fn asset_type_from_ogg() {
    assert_eq!(
        AssetType::from_path(Path::new("music.ogg")),
        AssetType::Audio
    );
}

#[test]
fn asset_type_from_mp3() {
    assert_eq!(AssetType::from_path(Path::new("sfx.mp3")), AssetType::Audio);
}

#[test]
fn asset_type_from_prefab_ron() {
    assert_eq!(
        AssetType::from_path(Path::new("tree.prefab.ron")),
        AssetType::Prefab
    );
}

#[test]
fn asset_type_from_unknown_ext() {
    assert_eq!(
        AssetType::from_path(Path::new("data.xyz")),
        AssetType::Unknown
    );
}

#[test]
fn asset_type_is_content_model() {
    assert!(AssetType::Model.is_content());
}

#[test]
fn asset_type_is_content_texture() {
    assert!(AssetType::Texture.is_content());
}

#[test]
fn asset_type_directory_not_content() {
    assert!(!AssetType::Directory.is_content());
}

#[test]
fn asset_type_config_not_content() {
    assert!(!AssetType::Config.is_content());
}

#[test]
fn asset_type_unknown_not_content() {
    assert!(!AssetType::Unknown.is_content());
}

#[test]
fn asset_type_all_count() {
    assert_eq!(AssetType::all().len(), 10);
}

// ============================================================================
// ASSET ACTION — METHODS
// ============================================================================

#[test]
fn asset_action_import_model_is_modifying() {
    let a = AssetAction::ImportModel {
        path: PathBuf::from("m.glb"),
    };
    assert!(a.is_modifying());
    assert!(!a.is_viewing());
    assert!(!a.is_scene_action());
}

#[test]
fn asset_action_load_viewport_is_viewing() {
    let a = AssetAction::LoadToViewport {
        path: PathBuf::from("m.glb"),
    };
    assert!(a.is_viewing());
    assert!(!a.is_modifying());
}

#[test]
fn asset_action_apply_texture_is_modifying() {
    let a = AssetAction::ApplyTexture {
        path: PathBuf::from("t.png"),
        texture_type: TextureType::Albedo,
    };
    assert!(a.is_modifying());
}

#[test]
fn asset_action_apply_material_is_modifying() {
    let a = AssetAction::ApplyMaterial {
        path: PathBuf::from("m.mat"),
    };
    assert!(a.is_modifying());
}

#[test]
fn asset_action_load_scene_is_scene_action() {
    let a = AssetAction::LoadScene {
        path: PathBuf::from("s.ron"),
    };
    assert!(a.is_scene_action());
    assert!(!a.is_modifying());
    assert!(!a.is_viewing());
}

#[test]
fn asset_action_spawn_prefab_is_modifying() {
    let a = AssetAction::SpawnPrefab {
        path: PathBuf::from("p.ron"),
    };
    assert!(a.is_modifying());
}

#[test]
fn asset_action_open_external_is_viewing() {
    let a = AssetAction::OpenExternal {
        path: PathBuf::from("f.glb"),
    };
    assert!(a.is_viewing());
}

#[test]
fn asset_action_inspect_asset_is_viewing() {
    let a = AssetAction::InspectAsset {
        path: PathBuf::from("a.glb"),
    };
    assert!(a.is_viewing());
}

#[test]
fn asset_action_path_accessor() {
    let p = PathBuf::from("test/mesh.glb");
    let a = AssetAction::ImportModel { path: p.clone() };
    assert_eq!(a.path(), &p);
}

#[test]
fn asset_action_name_nonempty() {
    let actions: Vec<AssetAction> = vec![
        AssetAction::ImportModel {
            path: PathBuf::from("a"),
        },
        AssetAction::LoadToViewport {
            path: PathBuf::from("a"),
        },
        AssetAction::ApplyTexture {
            path: PathBuf::from("a"),
            texture_type: TextureType::Normal,
        },
        AssetAction::ApplyMaterial {
            path: PathBuf::from("a"),
        },
        AssetAction::LoadScene {
            path: PathBuf::from("a"),
        },
        AssetAction::SpawnPrefab {
            path: PathBuf::from("a"),
        },
        AssetAction::OpenExternal {
            path: PathBuf::from("a"),
        },
        AssetAction::InspectAsset {
            path: PathBuf::from("a"),
        },
    ];
    for a in &actions {
        assert!(!a.name().is_empty());
        assert!(!a.icon().is_empty());
    }
}

#[test]
fn asset_action_all_variants_count() {
    assert_eq!(AssetAction::all_variants().len(), 10);
}

// ============================================================================
// VIEW MODE
// ============================================================================

#[test]
fn view_mode_all_count() {
    assert_eq!(ViewMode::all().len(), 2);
}

#[test]
fn view_mode_list_name() {
    assert_eq!(ViewMode::List.name(), "List");
}

#[test]
fn view_mode_grid_name() {
    assert_eq!(ViewMode::Grid.name(), "Grid");
}

#[test]
fn view_mode_list_icon() {
    assert_eq!(ViewMode::List.icon(), "📋");
}

#[test]
fn view_mode_grid_icon() {
    assert_eq!(ViewMode::Grid.icon(), "📰");
}
