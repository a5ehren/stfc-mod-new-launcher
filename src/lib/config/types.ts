export interface SyncTargetConfiguration {
	token: string;
	url: string;
	battlelogs: boolean;
	buffs: boolean;
	buildings: boolean;
	inventory: boolean;
	jobs: boolean;
	missions: boolean;
	officer: boolean;
	proxy: string;
	research: boolean;
	resources: boolean;
	ships: boolean;
	slots: boolean;
	tech: boolean;
	traits: boolean;
	[key: string]: unknown;
}

export type SyncTargetsConfiguration = Record<string, SyncTargetConfiguration>;

export type SyncConfiguration = SyncTargetConfiguration & {
	resolver_cache_ttl: number;
	verify_ssl: boolean;
	debug: boolean;
	logging: boolean;
	targets?: SyncTargetsConfiguration;
	[key: string]: unknown;
};

export interface BuffsConfiguration {
	use_out_of_dock_power: boolean;
	[key: string]: unknown;
}

export interface ConfigConfiguration {
	assets_url_override: string;
	settings_url: string;
	[key: string]: unknown;
}

export interface ControlConfiguration {
	enable_experimental: boolean;
	hotkeys_enabled: boolean;
	hotkeys_extended: boolean;
	use_scopely_hotkeys: boolean;
	queue_enabled: boolean;
	select_timer: number;
	[key: string]: unknown;
}

export interface GraphicsConfiguration {
	borderless_fullscreen: boolean;
	allow_cursor: boolean;
	default_system_zoom: number;
	free_resize: boolean;
	keyboard_zoom_speed: number;
	loader_enabled: boolean;
	loader_transition: boolean;
	loader_image: string;
	show_all_resolutions: boolean;
	system_pan_momentum_falloff: number;
	system_pan_momentum: number;
	system_zoom_preset_1: number;
	system_zoom_preset_2: number;
	system_zoom_preset_3: number;
	system_zoom_preset_4: number;
	system_zoom_preset_5: number;
	transition_time: number;
	ui_scale: number;
	ui_scale_adjust: number;
	ui_scale_viewer: number;
	use_presets_as_default: boolean;
	zoom: number;
	[key: string]: unknown;
}

export interface PatchesConfiguration {
	bufffixhooks: boolean;
	chatpatches: boolean;
	freeresizehooks: boolean;
	hotkeyhooks: boolean;
	improveresponsivenesshooks: boolean;
	miscpatches: boolean;
	objecttracker: boolean;
	panhooks: boolean;
	resolutionlistfix: boolean;
	syncpatches: boolean;
	tempcrashfixes: boolean;
	testpatches: boolean;
	toastbannerhooks: boolean;
	uiscalehooks: boolean;
	zoomhooks: boolean;
	[key: string]: unknown;
}

export interface ShortcutsConfiguration {
	toggle_queue: string;
	action_queue: string;
	action_queue_clear: string;
	action_primary: string;
	action_recall: string;
	action_recall_cancel: string;
	action_repair: string;
	action_secondary: string;
	action_view: string;
	set_hotkeys_disabled: string;
	set_hotkeys_enabled: string;
	log_off: string;
	log_error: string;
	log_warn: string;
	log_debug: string;
	log_info: string;
	log_trace: string;
	quit: string;
	select_chatalliance: string;
	select_chatglobal: string;
	select_chatprivate: string;
	select_current: string;
	select_ship1: string;
	select_ship2: string;
	select_ship3: string;
	select_ship4: string;
	select_ship5: string;
	select_ship6: string;
	select_ship7: string;
	select_ship8: string;
	set_zoom_default: string;
	set_zoom_preset1: string;
	set_zoom_preset2: string;
	set_zoom_preset3: string;
	set_zoom_preset4: string;
	set_zoom_preset5: string;
	show_alliance: string;
	show_alliance_armada: string;
	show_alliance_help: string;
	show_artifacts: string;
	show_awayteam: string;
	show_bookmarks: string;
	show_chat: string;
	show_chatside1: string;
	show_chatside2: string;
	show_commander: string;
	show_daily: string;
	show_events: string;
	show_exocomp: string;
	show_factions: string;
	show_galaxy: string;
	show_gifts: string;
	show_inventory: string;
	show_lookup: string;
	show_missions: string;
	show_officers: string;
	show_qtrials: string;
	show_refinery: string;
	show_research: string;
	show_scrapyard: string;
	show_settings: string;
	show_ships: string;
	show_stationexterior: string;
	show_stationinterior: string;
	show_system: string;
	toggle_cargo_armada: string;
	toggle_cargo_default: string;
	toggle_cargo_hostile: string;
	toggle_cargo_player: string;
	toggle_cargo_station: string;
	toggle_preview_locate: string;
	toggle_preview_recall: string;
	ui_scaledown: string;
	ui_scaleup: string;
	ui_scaleviewerdown: string;
	ui_scaleviewerup: string;
	zoom_in: string;
	zoom_max: string;
	zoom_min: string;
	zoom_out: string;
	zoom_reset: string;
	zoom_preset1: string;
	zoom_preset2: string;
	zoom_preset3: string;
	zoom_preset4: string;
	zoom_preset5: string;
	move_up: string;
	move_down: string;
	move_left: string;
	move_right: string;
	[key: string]: unknown;
}

export interface UiConfiguration {
	always_skip_reveal_sequence: boolean;
	auto_confirm_discovery: boolean;
	disable_escape_exit: boolean;
	disable_first_popup: boolean;
	disable_galaxy_chat: boolean;
	disable_move_keys: boolean;
	disable_preview_locate: boolean;
	disable_preview_recall: boolean;
	disable_toast_banners: boolean;
	disable_veil_chat: boolean;
	disabled_banner_types: string;
	extend_donation_max: number;
	extend_donation_slider: boolean;
	show_armada_cargo: boolean;
	show_cargo_default: boolean;
	show_hostile_cargo: boolean;
	show_player_cargo: boolean;
	show_station_cargo: boolean;
	[key: string]: unknown;
}

export interface TomlConfig {
	buffs: BuffsConfiguration;
	config: ConfigConfiguration;
	control: ControlConfiguration;
	graphics: GraphicsConfiguration;
	patches: PatchesConfiguration;
	shortcuts: ShortcutsConfiguration;
	ui: UiConfiguration;
	sync: SyncConfiguration;
	[key: string]: unknown;
}
