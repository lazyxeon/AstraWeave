use super::Panel;
use astraweave_core::{Entity, World};
use egui::Ui;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct HierarchyNode {
    pub entity: Entity,
    pub children: Vec<Entity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HierarchyAction {
    CreatePrefab(Entity),
    DeleteEntity(Entity),
    DuplicateEntity(Entity),
    FocusEntity(Entity),
    /// Week 5 Day 3-4: Break prefab connection (make entity standalone)
    BreakPrefabConnection(Entity),
    /// Week 5 Day 3-4: Apply prefab overrides to source file
    ApplyOverridesToPrefab(Entity),
    /// Week 5 Day 3-4: Revert overrides to original prefab values
    RevertToOriginalPrefab(Entity),
    /// Set parent of child entity (child, parent)
    SetParent(Entity, Entity),
    /// Remove entity from its parent (make it a root)
    Unparent(Entity),
    /// Open the prefab source for in-place editing
    EditPrefab(Entity),
    /// Rename entity (entity, new_name)
    Rename(Entity, String),
    /// Delete entity and all descendants
    DeleteWithChildren(Entity),
    /// Move entity up among siblings
    MoveUp(Entity),
    /// Move entity down among siblings
    MoveDown(Entity),
}

impl std::fmt::Display for HierarchyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl HierarchyAction {
    pub fn name(&self) -> &'static str {
        match self {
            HierarchyAction::CreatePrefab(_) => "Create Prefab",
            HierarchyAction::DeleteEntity(_) => "Delete Entity",
            HierarchyAction::DuplicateEntity(_) => "Duplicate Entity",
            HierarchyAction::FocusEntity(_) => "Focus Entity",
            HierarchyAction::BreakPrefabConnection(_) => "Break Prefab Connection",
            HierarchyAction::ApplyOverridesToPrefab(_) => "Apply Overrides",
            HierarchyAction::RevertToOriginalPrefab(_) => "Revert to Original",
            HierarchyAction::SetParent(_, _) => "Set Parent",
            HierarchyAction::Unparent(_) => "Unparent",
            HierarchyAction::EditPrefab(_) => "Edit Prefab",
            HierarchyAction::Rename(_, _) => "Rename",
            HierarchyAction::DeleteWithChildren(_) => "Delete With Children",
            HierarchyAction::MoveUp(_) => "Move Up",
            HierarchyAction::MoveDown(_) => "Move Down",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            HierarchyAction::CreatePrefab(_) => "[Pkg]",
            HierarchyAction::DeleteEntity(_) => "[Del]",
            HierarchyAction::DuplicateEntity(_) => "[List]",
            HierarchyAction::FocusEntity(_) => "[Tgt]",
            HierarchyAction::BreakPrefabConnection(_) => "[Link]",
            HierarchyAction::ApplyOverridesToPrefab(_) => "[ok]",
            HierarchyAction::RevertToOriginalPrefab(_) => "↩️",
            HierarchyAction::SetParent(_, _) => "[P+]",
            HierarchyAction::Unparent(_) => "[P-]",
            HierarchyAction::EditPrefab(_) => "[Ed]",
            HierarchyAction::Rename(_, _) => "[Rn]",
            HierarchyAction::DeleteWithChildren(_) => "[Dx]",
            HierarchyAction::MoveUp(_) => "[Up]",
            HierarchyAction::MoveDown(_) => "[Dn]",
        }
    }

    /// Returns true if this action modifies the entity
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            HierarchyAction::DeleteEntity(_)
                | HierarchyAction::DeleteWithChildren(_)
                | HierarchyAction::BreakPrefabConnection(_)
        )
    }

    /// Returns true if this action is prefab-related
    pub fn is_prefab_action(&self) -> bool {
        matches!(
            self,
            HierarchyAction::CreatePrefab(_)
                | HierarchyAction::BreakPrefabConnection(_)
                | HierarchyAction::ApplyOverridesToPrefab(_)
                | HierarchyAction::RevertToOriginalPrefab(_)
        )
    }
}

pub struct HierarchyPanel {
    hierarchy: HashMap<Entity, HierarchyNode>,
    root_entities: Vec<Entity>,

    selected_entities: HashSet<Entity>,
    last_clicked: Option<Entity>,

    drag_source: Option<Entity>,
    rename_entity: Option<Entity>,
    rename_buffer: String,

    context_menu_entity: Option<Entity>,
    empty_node_counter: u32,

    pending_actions: Vec<HierarchyAction>,
    search_filter: String,

    /// Week 5 Day 3-4: Track which entities are prefab instances
    prefab_instances: HashSet<Entity>,

    /// Entities hidden from viewport rendering (eye icon toggle).
    hidden_entities: HashSet<Entity>,
    /// Entities locked from selection/transform (lock icon toggle).
    locked_entities: HashSet<Entity>,
}

impl HierarchyPanel {
    pub fn new() -> Self {
        Self {
            hierarchy: HashMap::new(),
            root_entities: Vec::new(),
            selected_entities: HashSet::new(),
            last_clicked: None,
            drag_source: None,
            rename_entity: None,
            rename_buffer: String::new(),
            context_menu_entity: None,
            empty_node_counter: 0,
            pending_actions: Vec::new(),
            search_filter: String::new(),
            prefab_instances: HashSet::new(),
            hidden_entities: HashSet::new(),
            locked_entities: HashSet::new(),
        }
    }

    /// Check if an entity is hidden from viewport.
    pub fn is_hidden(&self, entity: Entity) -> bool {
        self.hidden_entities.contains(&entity)
    }

    /// Get all hidden entities (for viewport filtering).
    pub fn hidden_entities(&self) -> &HashSet<Entity> {
        &self.hidden_entities
    }

    /// Check if an entity is locked from selection/transform.
    pub fn is_locked(&self, entity: Entity) -> bool {
        self.locked_entities.contains(&entity)
    }

    /// Get all locked entities.
    pub fn locked_entities(&self) -> &HashSet<Entity> {
        &self.locked_entities
    }

    /// Week 5 Day 3-4: Mark an entity as a prefab instance
    pub fn mark_as_prefab_instance(&mut self, entity: Entity) {
        self.prefab_instances.insert(entity);
    }

    /// Week 5 Day 3-4: Unmark an entity as a prefab instance (after breaking connection)
    pub fn unmark_as_prefab_instance(&mut self, entity: Entity) {
        self.prefab_instances.remove(&entity);
    }

    /// Week 5 Day 3-4: Check if entity is a prefab instance
    pub fn is_prefab_instance(&self, entity: Entity) -> bool {
        self.prefab_instances.contains(&entity)
    }

    /// Week 5 Day 3-4: Sync prefab instances from prefab manager
    pub fn sync_prefab_instances(&mut self, prefab_entities: impl Iterator<Item = Entity>) {
        self.prefab_instances.clear();
        for entity in prefab_entities {
            self.prefab_instances.insert(entity);
        }
    }

    pub fn take_pending_actions(&mut self) -> Vec<HierarchyAction> {
        std::mem::take(&mut self.pending_actions)
    }

    pub fn sync_with_world(&mut self, world: &World) {
        let world_entities: HashSet<Entity> = world.entities().into_iter().collect();

        // Remove entities that no longer exist
        self.hierarchy.retain(|e, _| world_entities.contains(e));
        self.selected_entities
            .retain(|e| world_entities.contains(e));

        // Rebuild hierarchy from World's authoritative parent/children data
        for entity in world.entities() {
            let children: Vec<Entity> = world.children_of(entity).to_vec();
            self.hierarchy
                .entry(entity)
                .and_modify(|node| node.children = children.clone())
                .or_insert(HierarchyNode { entity, children });
        }

        // Rebuild root entities list: entities without a parent
        self.root_entities = world
            .entities()
            .into_iter()
            .filter(|e| world.parent_of(*e).is_none())
            .collect();
    }

    /// Rebuild the parent-child tree from EntityManager parent fields.
    /// Call after loading a scene to restore persisted hierarchy.
    pub fn rebuild_from_parents(&mut self, parent_map: &[(Entity, Option<Entity>)]) {
        // Clear all children lists
        for node in self.hierarchy.values_mut() {
            node.children.clear();
        }
        self.root_entities.clear();

        // Collect all known entities
        let all_entities: Vec<Entity> = self.hierarchy.keys().copied().collect();

        for &entity in &all_entities {
            // Find parent from the supplied map
            let parent = parent_map
                .iter()
                .find(|(e, _)| *e == entity)
                .and_then(|(_, p)| *p)
                .map(|pid| pid as Entity);

            if let Some(parent_id) = parent {
                if self.hierarchy.contains_key(&parent_id) && parent_id != entity {
                    if let Some(parent_node) = self.hierarchy.get_mut(&parent_id) {
                        parent_node.children.push(entity);
                    }
                } else {
                    // Parent doesn't exist or self-reference — make root
                    self.root_entities.push(entity);
                }
            } else {
                self.root_entities.push(entity);
            }
        }
    }

    pub fn get_selected(&self) -> Option<Entity> {
        self.selected_entities.iter().next().copied()
    }

    pub fn set_selected(&mut self, entity: Option<Entity>) {
        self.selected_entities.clear();
        if let Some(e) = entity {
            self.selected_entities.insert(e);
            self.last_clicked = Some(e);
        }
    }

    pub fn get_all_selected(&self) -> Vec<Entity> {
        self.selected_entities.iter().copied().collect()
    }

    pub fn set_selected_multiple(&mut self, entities: &[Entity]) {
        self.selected_entities.clear();
        for &e in entities {
            self.selected_entities.insert(e);
        }
        self.last_clicked = entities.first().copied();
    }

    fn add_child_to_parent(&mut self, child: Entity, parent: Entity) {
        if child == parent {
            return;
        }

        if self.is_ancestor_of(child, parent) {
            return;
        }

        self.remove_from_parent(child);

        if let Some(parent_node) = self.hierarchy.get_mut(&parent) {
            if !parent_node.children.contains(&child) {
                parent_node.children.push(child);
            }
        }

        self.root_entities.retain(|&e| e != child);
    }

    fn remove_from_parent(&mut self, child: Entity) {
        for node in self.hierarchy.values_mut() {
            node.children.retain(|&e| e != child);
        }

        if !self.root_entities.contains(&child) {
            self.root_entities.push(child);
        }
    }

    fn is_ancestor_of(&self, potential_ancestor: Entity, descendant: Entity) -> bool {
        if let Some(node) = self.hierarchy.get(&potential_ancestor) {
            for &child in &node.children {
                if child == descendant || self.is_ancestor_of(child, descendant) {
                    return true;
                }
            }
        }
        false
    }

    fn get_parent(&self, entity: Entity) -> Option<Entity> {
        for (parent, node) in &self.hierarchy {
            if node.children.contains(&entity) {
                return Some(*parent);
            }
        }
        None
    }

    fn entity_matches_filter(&self, world: &World, entity: Entity, filter_lower: &str) -> bool {
        if let Some(name) = world.name(entity) {
            if name.to_lowercase().contains(filter_lower) {
                return true;
            }
        }
        if let Some(node) = self.hierarchy.get(&entity) {
            for &child in &node.children {
                if self.entity_matches_filter(world, child, filter_lower) {
                    return true;
                }
            }
        }
        false
    }

    pub fn show_with_world(&mut self, ui: &mut Ui, world: &mut World) -> Option<Entity> {
        let mut selected_changed = None;

        ui.heading("Hierarchy");
        ui.separator();

        ui.horizontal(|ui| {
            // Add entity dropdown with archetypes
            ui.menu_button("+ Add", |ui| {
                ui.label(egui::RichText::new("Game Objects").strong().small());
                let archetypes: &[(&str, u8, i32, i32)] = &[
                    ("Empty", 0, 0, 0),
                    ("Player", 0, 100, 30),
                    ("Companion", 0, 80, 20),
                    ("Enemy", 1, 50, 20),
                    ("Boss", 1, 500, 50),
                    ("NPC", 2, 100, 0),
                    ("Prop", 2, 10, 0),
                ];
                for (name, team, hp, ammo) in archetypes {
                    if ui.button(format!("  {}", name)).clicked() {
                        self.empty_node_counter += 1;
                        let entity_name = format!("{}_{}", name, self.empty_node_counter);
                        let entity = world.spawn(
                            &entity_name,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                            astraweave_core::Team { id: *team },
                            *hp,
                            *ammo,
                        );
                        self.hierarchy.insert(
                            entity,
                            HierarchyNode {
                                entity,
                                children: Vec::new(),
                            },
                        );
                        self.root_entities.push(entity);
                        selected_changed = Some(entity);
                        ui.close();
                    }
                }

                ui.separator();
                ui.label(egui::RichText::new("Environment").strong().small());
                for name in &["Light", "Camera", "Trigger"] {
                    if ui.button(format!("  {}", name)).clicked() {
                        self.empty_node_counter += 1;
                        let entity_name = format!("{}_{}", name, self.empty_node_counter);
                        let entity = world.spawn(
                            &entity_name,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                            astraweave_core::Team { id: 2 },
                            1,
                            0,
                        );
                        self.hierarchy.insert(
                            entity,
                            HierarchyNode {
                                entity,
                                children: Vec::new(),
                            },
                        );
                        self.root_entities.push(entity);
                        selected_changed = Some(entity);
                        ui.close();
                    }
                }
            });

            if ui.button("Refresh").clicked() {
                self.sync_with_world(world);
            }
        });

        ui.add_space(5.0);

        if !self.selected_entities.is_empty() {
            ui.label(format!("Selected: {}", self.selected_entities.len()));
        }

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_filter)
                    .hint_text("Filter entities...")
                    .desired_width(ui.available_width() - 30.0),
            );
            if ui.button("X").clicked() {
                self.search_filter.clear();
            }
        });

        ui.add_space(5.0);

        let search_lower = self.search_filter.to_lowercase();
        let is_filtering = !self.search_filter.is_empty();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let root_entities = self.root_entities.clone();
                for entity in root_entities {
                    if is_filtering && !self.entity_matches_filter(world, entity, &search_lower) {
                        continue;
                    }
                    if let Some(new_selection) = self.show_entity_tree(ui, world, entity, 0) {
                        selected_changed = Some(new_selection);
                    }
                }
            });

        selected_changed
    }

    fn show_entity_tree(
        &mut self,
        ui: &mut Ui,
        world: &World,
        entity: Entity,
        depth: usize,
    ) -> Option<Entity> {
        let mut selected_changed = None;

        let name = world.name(entity).unwrap_or("Unknown");
        let is_selected = self.selected_entities.contains(&entity);
        let is_hidden = self.hidden_entities.contains(&entity);
        let is_locked = self.locked_entities.contains(&entity);

        let children = if let Some(node) = self.hierarchy.get(&entity) {
            node.children.clone()
        } else {
            Vec::new()
        };
        let has_children = !children.is_empty();

        let indent = depth as f32 * 16.0;

        ui.horizontal(|ui| {
            ui.add_space(indent);

            let (id, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 20.0));
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());

            if response.drag_started() {
                self.drag_source = Some(entity);
            }

            if response.drag_stopped() {
                if let Some(source) = self.drag_source.take() {
                    if source != entity {
                        self.add_child_to_parent(source, entity);
                        self.pending_actions
                            .push(HierarchyAction::SetParent(source, entity));
                    }
                }
            }

            if response.clicked() {
                let modifiers = ui.input(|i| i.modifiers);

                if modifiers.ctrl {
                    if self.selected_entities.contains(&entity) {
                        self.selected_entities.remove(&entity);
                    } else {
                        self.selected_entities.insert(entity);
                    }
                    self.last_clicked = Some(entity);
                } else if modifiers.shift {
                    if let Some(last) = self.last_clicked {
                        let all_entities = self.get_all_entities_in_tree();
                        if let (Some(start_idx), Some(end_idx)) = (
                            all_entities.iter().position(|&e| e == last),
                            all_entities.iter().position(|&e| e == entity),
                        ) {
                            let (min_idx, max_idx) = if start_idx < end_idx {
                                (start_idx, end_idx)
                            } else {
                                (end_idx, start_idx)
                            };
                            for &e in &all_entities[min_idx..=max_idx] {
                                self.selected_entities.insert(e);
                            }
                        }
                    }
                    self.last_clicked = Some(entity);
                } else {
                    self.selected_entities.clear();
                    self.selected_entities.insert(entity);
                    self.last_clicked = Some(entity);
                    selected_changed = Some(entity);
                }
            }

            if response.double_clicked() {
                self.pending_actions
                    .push(HierarchyAction::FocusEntity(entity));
            }

            response.context_menu(|ui| {
                self.context_menu_entity = Some(entity);

                if ui.button("[Edit] Rename").clicked() {
                    self.rename_entity = Some(entity);
                    self.rename_buffer = name.to_string();
                    ui.close();
                }

                if ui.button("Duplicate").clicked() {
                    self.pending_actions
                        .push(HierarchyAction::DuplicateEntity(entity));
                    ui.close();
                }

                if ui.button("Delete").clicked() {
                    self.pending_actions
                        .push(HierarchyAction::DeleteEntity(entity));
                    ui.close();
                }

                let has_kids = self
                    .hierarchy
                    .get(&entity)
                    .map(|n| !n.children.is_empty())
                    .unwrap_or(false);
                if has_kids && ui.button("Delete with Children").clicked() {
                    self.pending_actions
                        .push(HierarchyAction::DeleteWithChildren(entity));
                    ui.close();
                }

                ui.separator();

                if ui.button("Create Prefab").clicked() {
                    self.pending_actions
                        .push(HierarchyAction::CreatePrefab(entity));
                    ui.close();
                }

                // Week 5 Day 3-4: Prefab instance actions (only shown for prefab instances)
                let is_prefab = self.prefab_instances.contains(&entity);
                if is_prefab {
                    ui.separator();
                    ui.label("Prefab Instance");

                    if ui.button("[Up] Apply Overrides to Prefab").clicked() {
                        self.pending_actions
                            .push(HierarchyAction::ApplyOverridesToPrefab(entity));
                        ui.close();
                    }

                    if ui.button("[Dn] Revert to Original").clicked() {
                        self.pending_actions
                            .push(HierarchyAction::RevertToOriginalPrefab(entity));
                        ui.close();
                    }

                    if ui.button("Break Prefab Connection").clicked() {
                        self.pending_actions
                            .push(HierarchyAction::BreakPrefabConnection(entity));
                        ui.close();
                    }

                    if ui.button("✏️ Edit Prefab Source").clicked() {
                        self.pending_actions
                            .push(HierarchyAction::EditPrefab(entity));
                        ui.close();
                    }
                }

                ui.separator();

                if ui.button("Move Up").clicked() {
                    self.pending_actions.push(HierarchyAction::MoveUp(entity));
                    ui.close();
                }

                if ui.button("Move Down").clicked() {
                    self.pending_actions.push(HierarchyAction::MoveDown(entity));
                    ui.close();
                }

                if ui.button("Unparent").clicked() {
                    self.remove_from_parent(entity);
                    self.pending_actions.push(HierarchyAction::Unparent(entity));
                    ui.close();
                }
            });

            let painter = ui.painter_at(rect);

            if is_selected {
                painter.rect_filled(
                    rect,
                    2.0,
                    egui::Color32::from_rgb(50, 100, 200).linear_multiply(0.3),
                );
            } else if response.hovered() {
                painter.rect_filled(
                    rect,
                    2.0,
                    egui::Color32::from_rgb(255, 255, 255).linear_multiply(0.1),
                );
            }

            if self.drag_source.is_some() && response.hovered() {
                painter.rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    egui::StrokeKind::Outside,
                );
            }

            let text_pos = rect.min
                + egui::vec2(
                    if has_children { 20.0 } else { 10.0 },
                    rect.height() * 0.5 - 7.0,
                );

            if has_children {
                let arrow = if ui.memory(|mem| {
                    mem.data
                        .get_temp::<bool>(egui::Id::new(format!("collapse_{}", entity)))
                        .unwrap_or(false)
                }) {
                    "▼"
                } else {
                    ">"
                };
                painter.text(
                    rect.min + egui::vec2(5.0, rect.height() * 0.5 - 7.0),
                    egui::Align2::LEFT_TOP,
                    arrow,
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );

                let arrow_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(15.0, rect.height()));
                if ui
                    .interact(
                        arrow_rect,
                        egui::Id::new(format!("arrow_{}", entity)),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    let current = ui.memory(|mem| {
                        mem.data
                            .get_temp::<bool>(egui::Id::new(format!("collapse_{}", entity)))
                            .unwrap_or(false)
                    });
                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(egui::Id::new(format!("collapse_{}", entity)), !current)
                    });
                }
            }

            // Draw entity type icon
            let icon = Self::entity_type_icon(name);
            let is_prefab_inst = self.prefab_instances.contains(&entity);
            let icon_color = if is_prefab_inst {
                egui::Color32::from_rgb(100, 180, 255) // Blue for prefab instances
            } else {
                egui::Color32::from_rgb(120, 120, 120)
            };
            painter.text(
                text_pos - egui::vec2(if has_children { 0.0 } else { 0.0 }, 0.0),
                egui::Align2::LEFT_TOP,
                icon,
                egui::FontId::proportional(10.0),
                icon_color,
            );
            let icon_width = 24.0;
            let name_pos = text_pos + egui::vec2(icon_width, 0.0);

            if Some(entity) == self.rename_entity {
                let text_edit =
                    egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(150.0);
                let text_response = ui.put(
                    egui::Rect::from_min_size(text_pos, egui::vec2(150.0, 16.0)),
                    text_edit,
                );

                if text_response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !self.rename_buffer.is_empty() {
                        self.pending_actions
                            .push(HierarchyAction::Rename(entity, self.rename_buffer.clone()));
                    }
                    self.rename_entity = None;
                }
            } else {
                // Draw entity name with optional search highlighting
                let search_lower = self.search_filter.to_lowercase();
                let name_lower = name.to_lowercase();
                if !search_lower.is_empty() {
                    if let Some(match_start) = name_lower.find(&search_lower) {
                        // Draw name in three parts: before, highlighted, after
                        let before = &name[..match_start];
                        let matched = &name[match_start..match_start + search_lower.len()];
                        let after = &name[match_start + search_lower.len()..];

                        let text_color = if is_selected {
                            egui::Color32::WHITE
                        } else if is_hidden {
                            egui::Color32::from_rgb(100, 100, 100)
                        } else {
                            egui::Color32::LIGHT_GRAY
                        };

                        let mut x_offset = name_pos.x;
                        let font = egui::FontId::default();

                        if !before.is_empty() {
                            let galley = painter.layout_no_wrap(
                                before.to_string(),
                                font.clone(),
                                text_color,
                            );
                            let w = galley.rect.width();
                            painter.galley(
                                egui::Pos2::new(x_offset, name_pos.y),
                                galley,
                                text_color,
                            );
                            x_offset += w;
                        }

                        // Highlighted match
                        let galley = painter.layout_no_wrap(
                            matched.to_string(),
                            font.clone(),
                            egui::Color32::BLACK,
                        );
                        let w = galley.rect.width();
                        let highlight_rect = egui::Rect::from_min_size(
                            egui::Pos2::new(x_offset, name_pos.y),
                            egui::vec2(w, 14.0),
                        );
                        painter.rect_filled(
                            highlight_rect,
                            2.0,
                            egui::Color32::from_rgb(255, 200, 50),
                        );
                        painter.galley(
                            egui::Pos2::new(x_offset, name_pos.y),
                            galley,
                            egui::Color32::BLACK,
                        );
                        x_offset += w;

                        if !after.is_empty() {
                            let galley =
                                painter.layout_no_wrap(after.to_string(), font, text_color);
                            painter.galley(
                                egui::Pos2::new(x_offset, name_pos.y),
                                galley,
                                text_color,
                            );
                        }
                    } else {
                        painter.text(
                            name_pos,
                            egui::Align2::LEFT_TOP,
                            name,
                            egui::FontId::default(),
                            if is_selected {
                                egui::Color32::WHITE
                            } else if is_hidden {
                                egui::Color32::from_rgb(100, 100, 100)
                            } else {
                                egui::Color32::LIGHT_GRAY
                            },
                        );
                    }
                } else {
                    painter.text(
                        name_pos,
                        egui::Align2::LEFT_TOP,
                        name,
                        egui::FontId::default(),
                        if is_selected {
                            egui::Color32::WHITE
                        } else if is_hidden {
                            egui::Color32::from_rgb(100, 100, 100)
                        } else {
                            egui::Color32::LIGHT_GRAY
                        },
                    );
                }

                // Eye icon (visibility toggle)
                let icon_area_x = rect.max.x - 40.0;
                let eye_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(icon_area_x, rect.min.y + 2.0),
                    egui::vec2(16.0, 16.0),
                );
                let eye_resp = ui.interact(
                    eye_rect,
                    egui::Id::new(format!("eye_{}", entity)),
                    egui::Sense::click(),
                );
                let eye_icon = if is_hidden { "○" } else { "◉" };
                let eye_color = if is_hidden {
                    egui::Color32::from_rgb(80, 80, 80)
                } else if eye_resp.hovered() {
                    egui::Color32::from_rgb(180, 220, 255)
                } else {
                    egui::Color32::from_rgb(140, 140, 140)
                };
                painter.text(
                    eye_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    eye_icon,
                    egui::FontId::proportional(12.0),
                    eye_color,
                );
                if eye_resp.clicked() {
                    if is_hidden {
                        self.hidden_entities.remove(&entity);
                    } else {
                        self.hidden_entities.insert(entity);
                    }
                }

                // Lock icon (lock toggle)
                let lock_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(icon_area_x + 18.0, rect.min.y + 2.0),
                    egui::vec2(16.0, 16.0),
                );
                let lock_resp = ui.interact(
                    lock_rect,
                    egui::Id::new(format!("lock_{}", entity)),
                    egui::Sense::click(),
                );
                let lock_icon = if is_locked { "■" } else { "□" };
                let lock_color = if is_locked {
                    egui::Color32::from_rgb(255, 160, 60)
                } else if lock_resp.hovered() {
                    egui::Color32::from_rgb(180, 180, 180)
                } else {
                    egui::Color32::from_rgb(100, 100, 100)
                };
                painter.text(
                    lock_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    lock_icon,
                    egui::FontId::proportional(12.0),
                    lock_color,
                );
                if lock_resp.clicked() {
                    if is_locked {
                        self.locked_entities.remove(&entity);
                    } else {
                        self.locked_entities.insert(entity);
                    }
                }
            }
        });

        if has_children {
            let is_expanded = ui.memory(|mem| {
                mem.data
                    .get_temp::<bool>(egui::Id::new(format!("collapse_{}", entity)))
                    .unwrap_or(false)
            });
            if is_expanded {
                for child in children {
                    if let Some(new_selection) = self.show_entity_tree(ui, world, child, depth + 1)
                    {
                        selected_changed = Some(new_selection);
                    }
                }
            }
        }

        selected_changed
    }

    /// Infer entity type icon from name prefix.
    fn entity_type_icon(name: &str) -> &'static str {
        let lower = name.to_lowercase();
        if lower.starts_with("light") {
            "[L]"
        } else if lower.starts_with("camera") {
            "[C]"
        } else if lower.starts_with("player") {
            "[P]"
        } else if lower.starts_with("enemy") || lower.starts_with("boss") {
            "[E]"
        } else if lower.starts_with("companion") || lower.starts_with("npc") {
            "[N]"
        } else if lower.starts_with("trigger") {
            "[T]"
        } else if lower.starts_with("prop") {
            "[M]"
        } else {
            "[O]"
        }
    }

    /// Move entity up among its siblings.
    pub fn move_entity_up(&mut self, entity: Entity) {
        let parent = self.get_parent(entity);
        let siblings = if let Some(p) = parent {
            if let Some(node) = self.hierarchy.get_mut(&p) {
                &mut node.children
            } else {
                return;
            }
        } else {
            &mut self.root_entities
        };
        if let Some(idx) = siblings.iter().position(|&e| e == entity) {
            if idx > 0 {
                siblings.swap(idx, idx - 1);
            }
        }
    }

    /// Move entity down among its siblings.
    pub fn move_entity_down(&mut self, entity: Entity) {
        let parent = self.get_parent(entity);
        let siblings = if let Some(p) = parent {
            if let Some(node) = self.hierarchy.get_mut(&p) {
                &mut node.children
            } else {
                return;
            }
        } else {
            &mut self.root_entities
        };
        if let Some(idx) = siblings.iter().position(|&e| e == entity) {
            if idx + 1 < siblings.len() {
                siblings.swap(idx, idx + 1);
            }
        }
    }

    fn get_all_entities_in_tree(&self) -> Vec<Entity> {
        let mut result = Vec::new();
        for &entity in &self.root_entities {
            self.collect_entities_recursive(entity, &mut result);
        }
        result
    }

    fn collect_entities_recursive(&self, entity: Entity, result: &mut Vec<Entity>) {
        result.push(entity);
        if let Some(node) = self.hierarchy.get(&entity) {
            for &child in &node.children {
                self.collect_entities_recursive(child, result);
            }
        }
    }
}

impl Default for HierarchyPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for HierarchyPanel {
    fn name(&self) -> &str {
        "Hierarchy"
    }

    fn show(&mut self, ui: &mut Ui) {
        ui.heading("🌲 Hierarchy");
        ui.label("Hierarchy panel requires world integration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_panel_new() {
        let panel = HierarchyPanel::new();
        assert_eq!(panel.root_entities.len(), 0);
        assert_eq!(panel.selected_entities.len(), 0);
        assert!(panel.hidden_entities.is_empty());
        assert!(panel.locked_entities.is_empty());
    }

    #[test]
    fn test_add_child_to_parent() {
        let mut panel = HierarchyPanel::new();

        panel.hierarchy.insert(
            1,
            HierarchyNode {
                entity: 1,
                children: Vec::new(),
            },
        );
        panel.hierarchy.insert(
            2,
            HierarchyNode {
                entity: 2,
                children: Vec::new(),
            },
        );
        panel.root_entities = vec![1, 2];

        panel.add_child_to_parent(2, 1);

        assert_eq!(panel.hierarchy.get(&1).unwrap().children, vec![2]);
        assert!(!panel.root_entities.contains(&2));
        assert_eq!(panel.root_entities, vec![1]);
    }

    #[test]
    fn test_prevent_circular_parenting() {
        let mut panel = HierarchyPanel::new();

        panel.hierarchy.insert(
            1,
            HierarchyNode {
                entity: 1,
                children: vec![2],
            },
        );
        panel.hierarchy.insert(
            2,
            HierarchyNode {
                entity: 2,
                children: vec![3],
            },
        );
        panel.hierarchy.insert(
            3,
            HierarchyNode {
                entity: 3,
                children: Vec::new(),
            },
        );

        panel.add_child_to_parent(1, 3);

        assert!(!panel.hierarchy.get(&3).unwrap().children.contains(&1));
    }

    #[test]
    fn test_remove_from_parent() {
        let mut panel = HierarchyPanel::new();

        panel.hierarchy.insert(
            1,
            HierarchyNode {
                entity: 1,
                children: vec![2],
            },
        );
        panel.hierarchy.insert(
            2,
            HierarchyNode {
                entity: 2,
                children: Vec::new(),
            },
        );
        panel.root_entities = vec![1];

        panel.remove_from_parent(2);

        assert_eq!(panel.hierarchy.get(&1).unwrap().children.len(), 0);
        assert!(panel.root_entities.contains(&2));
    }

    #[test]
    fn test_multi_selection() {
        let mut panel = HierarchyPanel::new();

        panel.selected_entities.insert(1);
        panel.selected_entities.insert(2);
        panel.selected_entities.insert(3);

        assert_eq!(panel.get_all_selected().len(), 3);
    }

    #[test]
    fn test_sync_with_world_removes_deleted_entities() {
        let mut panel = HierarchyPanel::new();
        let mut world = World::new();

        let e1 = world.spawn(
            "Entity1",
            astraweave_core::IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            100,
            10,
        );

        panel.hierarchy.insert(
            e1,
            HierarchyNode {
                entity: e1,
                children: Vec::new(),
            },
        );
        panel.hierarchy.insert(
            999,
            HierarchyNode {
                entity: 999,
                children: Vec::new(),
            },
        );
        panel.root_entities = vec![e1, 999];

        panel.sync_with_world(&world);

        assert!(panel.hierarchy.contains_key(&e1));
        assert!(!panel.hierarchy.contains_key(&999));
        assert!(!panel.root_entities.contains(&999));
    }

    #[test]
    fn test_prefab_instance_tracking() {
        let mut panel = HierarchyPanel::new();
        panel.mark_as_prefab_instance(1);
        assert!(panel.is_prefab_instance(1));
        assert!(!panel.is_prefab_instance(2));

        panel.unmark_as_prefab_instance(1);
        assert!(!panel.is_prefab_instance(1));

        let prefabs = vec![10, 20, 30];
        panel.sync_prefab_instances(prefabs.into_iter());
        assert!(panel.is_prefab_instance(10));
        assert!(panel.is_prefab_instance(20));
        assert!(panel.is_prefab_instance(30));
    }

    #[test]
    fn test_selection_helpers() {
        let mut panel = HierarchyPanel::new();

        // Single selection
        panel.set_selected(Some(1));
        assert_eq!(panel.get_selected(), Some(1));
        assert_eq!(panel.get_all_selected().len(), 1);

        // Clear
        panel.set_selected(None);
        assert!(panel.get_selected().is_none());
        assert!(panel.get_all_selected().is_empty());

        // Multiple
        panel.set_selected_multiple(&[1, 2, 3]);
        let selected = panel.get_all_selected();
        assert_eq!(selected.len(), 3);
        assert!(selected.contains(&1));
        assert!(selected.contains(&2));
        assert!(selected.contains(&3));
    }

    #[test]
    fn test_pending_actions() {
        let mut panel = HierarchyPanel::new();
        panel.pending_actions.push(HierarchyAction::DeleteEntity(1));
        panel.pending_actions.push(HierarchyAction::FocusEntity(2));

        let actions = panel.take_pending_actions();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], HierarchyAction::DeleteEntity(1));
        assert!(panel.pending_actions.is_empty());
    }

    #[test]
    fn test_get_parent() {
        let mut panel = HierarchyPanel::new();
        // 1 -> 2
        panel.hierarchy.insert(
            1,
            HierarchyNode {
                entity: 1,
                children: vec![2],
            },
        );
        panel.hierarchy.insert(
            2,
            HierarchyNode {
                entity: 2,
                children: vec![],
            },
        );

        assert_eq!(panel.get_parent(2), Some(1));
        assert_eq!(panel.get_parent(1), None);
    }

    #[test]
    fn test_is_ancestor_recursion() {
        let mut panel = HierarchyPanel::new();
        // 1 -> 2 -> 3
        panel.hierarchy.insert(
            1,
            HierarchyNode {
                entity: 1,
                children: vec![2],
            },
        );
        panel.hierarchy.insert(
            2,
            HierarchyNode {
                entity: 2,
                children: vec![3],
            },
        );
        panel.hierarchy.insert(
            3,
            HierarchyNode {
                entity: 3,
                children: vec![],
            },
        );

        assert!(panel.is_ancestor_of(1, 3));
        assert!(panel.is_ancestor_of(1, 2));
        assert!(panel.is_ancestor_of(2, 3));
        assert!(!panel.is_ancestor_of(3, 1));
    }

    // ===== HierarchyAction Enum Tests =====

    #[test]
    fn test_hierarchy_action_display() {
        let action = HierarchyAction::CreatePrefab(1);
        let display = format!("{}", action);
        assert!(
            display.contains(action.name()),
            "Display should contain name"
        );
    }

    #[test]
    fn test_hierarchy_action_name() {
        assert_eq!(HierarchyAction::CreatePrefab(1).name(), "Create Prefab");
        assert_eq!(HierarchyAction::DeleteEntity(1).name(), "Delete Entity");
        assert_eq!(
            HierarchyAction::DuplicateEntity(1).name(),
            "Duplicate Entity"
        );
        assert_eq!(HierarchyAction::FocusEntity(1).name(), "Focus Entity");
    }

    #[test]
    fn test_hierarchy_action_icon() {
        let action = HierarchyAction::CreatePrefab(1);
        assert!(!action.icon().is_empty(), "Icon should not be empty");
    }

    #[test]
    fn test_hierarchy_action_is_destructive() {
        assert!(HierarchyAction::DeleteEntity(1).is_destructive());
        assert!(HierarchyAction::BreakPrefabConnection(1).is_destructive());
        assert!(!HierarchyAction::CreatePrefab(1).is_destructive());
        assert!(!HierarchyAction::FocusEntity(1).is_destructive());
    }

    #[test]
    fn test_hierarchy_action_is_prefab_action() {
        assert!(HierarchyAction::CreatePrefab(1).is_prefab_action());
        assert!(HierarchyAction::BreakPrefabConnection(1).is_prefab_action());
        assert!(HierarchyAction::ApplyOverridesToPrefab(1).is_prefab_action());
        assert!(HierarchyAction::RevertToOriginalPrefab(1).is_prefab_action());
        assert!(!HierarchyAction::DeleteEntity(1).is_prefab_action());
    }

    #[test]
    fn test_hierarchy_action_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(HierarchyAction::CreatePrefab(1));
        set.insert(HierarchyAction::DeleteEntity(1));
        set.insert(HierarchyAction::CreatePrefab(1)); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ===== Visibility & Lock Tests =====

    #[test]
    fn test_visibility_toggle() {
        let mut panel = HierarchyPanel::new();
        assert!(!panel.is_hidden(1));

        panel.hidden_entities.insert(1);
        assert!(panel.is_hidden(1));
        assert!(!panel.is_hidden(2));

        panel.hidden_entities.remove(&1);
        assert!(!panel.is_hidden(1));
    }

    #[test]
    fn test_lock_toggle() {
        let mut panel = HierarchyPanel::new();
        assert!(!panel.is_locked(1));

        panel.locked_entities.insert(1);
        assert!(panel.is_locked(1));
        assert!(!panel.is_locked(2));

        panel.locked_entities.remove(&1);
        assert!(!panel.is_locked(1));
    }

    #[test]
    fn test_hidden_entities_accessor() {
        let mut panel = HierarchyPanel::new();
        panel.hidden_entities.insert(1);
        panel.hidden_entities.insert(3);

        let hidden = panel.hidden_entities();
        assert_eq!(hidden.len(), 2);
        assert!(hidden.contains(&1));
        assert!(hidden.contains(&3));
    }

    #[test]
    fn test_locked_entities_accessor() {
        let mut panel = HierarchyPanel::new();
        panel.locked_entities.insert(5);

        let locked = panel.locked_entities();
        assert_eq!(locked.len(), 1);
        assert!(locked.contains(&5));
    }
}
