// cardinal-xr/src/hand_menu.rs
//! Hand menu state management and catalog grouping logic.

use cardinal_core::CatalogEntry;
use glam::Vec3;
use rustc_hash::FxHashMap;

use crate::constants::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuLevel {
    Tag,
    Plugin,
    Module,
}

pub struct PluginGroup {
    pub plugin_slug: String,
    pub display_name: String,
    pub modules: Vec<ModuleEntry>,
}

pub struct ModuleEntry {
    pub model_slug: String,
    pub display_name: String,
    pub plugin_slug: String,
    pub tags: Vec<String>,
}

pub struct HandMenuState {
    pub all_plugins: Vec<PluginGroup>,
    pub tags: Vec<String>,
    pub selected_tag: Option<usize>,
    pub filtered_plugins: Vec<PluginGroup>,
    pub selected_plugin: Option<usize>,
    pub filtered_modules: Vec<ModuleEntry>,
    pub hover_timers: FxHashMap<(MenuLevel, usize), f32>,
    pub visible: bool,
    pub smoothed_position: Vec3,
    pub scroll_offsets: FxHashMap<MenuLevel, usize>,
}

impl HandMenuState {
    pub fn from_catalog(catalog: &[CatalogEntry]) -> Self {
        // Group entries by plugin_slug.
        let mut plugin_map: std::collections::BTreeMap<String, Vec<&CatalogEntry>> =
            std::collections::BTreeMap::new();
        for entry in catalog {
            plugin_map
                .entry(entry.plugin_slug.clone())
                .or_default()
                .push(entry);
        }

        // Build sorted PluginGroup list (BTreeMap gives alphabetical key order).
        let all_plugins: Vec<PluginGroup> = plugin_map
            .into_iter()
            .map(|(plugin_slug, entries)| {
                let display_name = plugin_slug.clone();
                let mut modules: Vec<ModuleEntry> = entries
                    .into_iter()
                    .map(|e| ModuleEntry {
                        model_slug: e.model_slug.clone(),
                        display_name: e.model_name.clone(),
                        plugin_slug: e.plugin_slug.clone(),
                        tags: vec![],
                    })
                    .collect();
                modules.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                PluginGroup {
                    plugin_slug,
                    display_name,
                    modules,
                }
            })
            .collect();

        // filtered_plugins starts as a clone of all_plugins.
        let filtered_plugins = clone_plugins(&all_plugins);

        Self {
            all_plugins,
            tags: vec![],
            selected_tag: None,
            filtered_plugins,
            selected_plugin: None,
            filtered_modules: vec![],
            hover_timers: FxHashMap::default(),
            visible: false,
            smoothed_position: Vec3::ZERO,
            scroll_offsets: FxHashMap::default(),
        }
    }

    /// Show/hide based on palm-up amount with hysteresis.
    pub fn update_palm_visibility(&mut self, palm_up_amount: f32) {
        if !self.visible && palm_up_amount >= MENU_PALM_UP_THRESHOLD {
            self.visible = true;
        } else if self.visible && palm_up_amount < MENU_PALM_DOWN_THRESHOLD {
            self.close();
        }
    }

    /// Close the menu and reset all transient state.
    pub fn close(&mut self) {
        self.visible = false;
        self.selected_tag = None;
        self.selected_plugin = None;
        self.filtered_modules = vec![];
        self.hover_timers = FxHashMap::default();
        self.scroll_offsets = FxHashMap::default();
        self.refilter_plugins();
    }

    /// Select a tag and reset plugin selection.
    pub fn select_tag(&mut self, tag_idx: Option<usize>) {
        self.selected_tag = tag_idx;
        self.selected_plugin = None;
        self.filtered_modules = vec![];
        self.refilter_plugins();
    }

    /// Select a plugin and populate filtered_modules from it.
    pub fn select_plugin(&mut self, plugin_idx: Option<usize>) {
        self.selected_plugin = plugin_idx;
        self.filtered_modules = match plugin_idx {
            Some(idx) => self
                .filtered_plugins
                .get(idx)
                .map(|pg| clone_modules(&pg.modules))
                .unwrap_or_default(),
            None => vec![],
        };
    }

    /// Increment hover timer. Returns `true` exactly the first time it crosses
    /// the `MENU_HOVER_EXPAND_DELAY_SECS` threshold.
    pub fn update_hover(&mut self, level: MenuLevel, index: usize, dt: f32) -> bool {
        let key = (level, index);
        let timer = self.hover_timers.entry(key).or_insert(0.0);
        let was_below = *timer < MENU_HOVER_EXPAND_DELAY_SECS;
        *timer += dt;
        let now_above = *timer >= MENU_HOVER_EXPAND_DELAY_SECS;
        was_below && now_above
    }

    /// Remove hover timer for a specific entry.
    pub fn reset_hover(&mut self, level: MenuLevel, index: usize) {
        self.hover_timers.remove(&(level, index));
    }

    /// Remove all hover timers for a given level.
    pub fn reset_level_hovers(&mut self, level: MenuLevel) {
        self.hover_timers.retain(|(l, _), _| *l != level);
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn refilter_plugins(&mut self) {
        // Tags are not yet implemented; filtered_plugins mirrors all_plugins.
        self.filtered_plugins = clone_plugins(&self.all_plugins);
    }
}

fn clone_modules(modules: &[ModuleEntry]) -> Vec<ModuleEntry> {
    modules
        .iter()
        .map(|m| ModuleEntry {
            model_slug: m.model_slug.clone(),
            display_name: m.display_name.clone(),
            plugin_slug: m.plugin_slug.clone(),
            tags: m.tags.clone(),
        })
        .collect()
}

fn clone_plugins(plugins: &[PluginGroup]) -> Vec<PluginGroup> {
    plugins
        .iter()
        .map(|pg| PluginGroup {
            plugin_slug: pg.plugin_slug.clone(),
            display_name: pg.display_name.clone(),
            modules: clone_modules(&pg.modules),
        })
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> Vec<CatalogEntry> {
        vec![
            CatalogEntry {
                plugin_slug: "Fundamental".into(),
                model_slug: "VCO".into(),
                model_name: "VCO".into(),
            },
            CatalogEntry {
                plugin_slug: "Fundamental".into(),
                model_slug: "VCF".into(),
                model_name: "VCF".into(),
            },
            CatalogEntry {
                plugin_slug: "Befaco".into(),
                model_slug: "Mixer".into(),
                model_name: "Mixer".into(),
            },
        ]
    }

    #[test]
    fn test_catalog_grouping() {
        let catalog = sample_catalog();
        let state = HandMenuState::from_catalog(&catalog);

        assert_eq!(state.all_plugins.len(), 2, "should have 2 plugin groups");

        // Alphabetical: Befaco before Fundamental
        assert_eq!(state.all_plugins[0].plugin_slug, "Befaco");
        assert_eq!(state.all_plugins[1].plugin_slug, "Fundamental");

        assert_eq!(state.all_plugins[1].modules.len(), 2, "Fundamental has 2 modules");
        assert_eq!(state.filtered_plugins.len(), 2);
        assert!(!state.visible);
    }

    #[test]
    fn test_plugin_selection_populates_modules() {
        let catalog = sample_catalog();
        let mut state = HandMenuState::from_catalog(&catalog);

        // Index 1 is Fundamental
        state.select_plugin(Some(1));

        assert_eq!(state.selected_plugin, Some(1));
        assert_eq!(state.filtered_modules.len(), 2);
        // Sorted alphabetically: VCF before VCO
        assert_eq!(state.filtered_modules[0].model_slug, "VCF");
        assert_eq!(state.filtered_modules[1].model_slug, "VCO");
    }

    #[test]
    fn test_palm_hysteresis() {
        let catalog = sample_catalog();
        let mut state = HandMenuState::from_catalog(&catalog);

        // Below open threshold → stays hidden
        state.update_palm_visibility(0.5);
        assert!(!state.visible);

        // Above open threshold → visible
        state.update_palm_visibility(0.8);
        assert!(state.visible);

        // Between thresholds → stays visible (hysteresis)
        state.update_palm_visibility(0.6);
        assert!(state.visible, "should stay visible between thresholds");

        // Below close threshold → hidden
        state.update_palm_visibility(0.4);
        assert!(!state.visible);
    }

    #[test]
    fn test_hover_timer() {
        let catalog = sample_catalog();
        let mut state = HandMenuState::from_catalog(&catalog);
        let level = MenuLevel::Plugin;

        // Accumulate below threshold → false
        let result = state.update_hover(level, 0, 0.1);
        assert!(!result, "should not trigger below threshold");

        // Accumulate past threshold → true (first crossing: 0.1 + 0.25 = 0.35 >= 0.3)
        let result = state.update_hover(level, 0, 0.25);
        assert!(result, "should trigger on first crossing");

        // Already past threshold → false (no re-trigger)
        let result = state.update_hover(level, 0, 0.1);
        assert!(!result, "should not re-trigger after crossing");

        // Reset then accumulate → false again
        state.reset_hover(level, 0);
        let result = state.update_hover(level, 0, 0.1);
        assert!(!result, "should not trigger after reset with small dt");
    }

    #[test]
    fn test_close_resets_state() {
        let catalog = sample_catalog();
        let mut state = HandMenuState::from_catalog(&catalog);

        // Set up some state
        state.visible = true;
        state.select_tag(None); // no-op for tags but exercises code path
        state.select_plugin(Some(1));
        state.update_hover(MenuLevel::Plugin, 0, 0.5);
        state.scroll_offsets.insert(MenuLevel::Module, 3);

        // Close should reset everything
        state.close();

        assert!(!state.visible);
        assert_eq!(state.selected_tag, None);
        assert_eq!(state.selected_plugin, None);
        assert!(state.filtered_modules.is_empty());
        assert!(state.hover_timers.is_empty());
        assert!(state.scroll_offsets.is_empty());
        // filtered_plugins should be repopulated
        assert_eq!(state.filtered_plugins.len(), 2);
    }
}
