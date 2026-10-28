// Copyright (C) 2026 Torkel Danielsson
// SPDX-License-Identifier: MIT

use oden_plugin_rs::{
    GuiParams, InitParams, OdenPlugin, SceneApi, ShutdownParams, UpdateParams,
    log::{info, warn},
    math::Vec4,
    register_plugin,
};
use std::time::Instant;

const INPUT_ENTITY_TYPES: &[&str] = &[
    "Stitched Video",
    "Equirectangular Video",
    "2D Video",
    "ZCam",
    "Meta",
    "Kandao",
    "Insta360",
];

const INPUT_KEY: &str = "selected_entity_uuid";
const STREAM_KEY: &str = "stream_id";
const TIMEOUT_KEY: &str = "timeout_seconds";
const TARGET_SCENE_KEY: &str = "selected_scene_id";
const ACTIVE_KEY: &str = "active";

const NO_SCENE: i32 = -1;
const MIN_TIMEOUT_S: f32 = 0.1;
const MAX_TIMEOUT_S: f32 = 5.0;

const GREEN: Vec4 = Vec4::rgba(0.0, 1.0, 0.0, 1.0);
const YELLOW: Vec4 = Vec4::rgba(1.0, 1.0, 0.0, 1.0);
const RED: Vec4 = Vec4::rgba(1.0, 0.3, 0.3, 1.0);
const DIM: Vec4 = Vec4::rgba(1.0, 1.0, 1.0, 0.5);

struct SceneChangeOnLostInput {
    input: String,
    stream: i32,
    timeout_s: f32,
    target_scene: i32,
    active: bool,
    last_frame_time: Option<u64>,
    last_frame_arrived: Instant,
    switched: bool,
}

impl SceneChangeOnLostInput {
    const fn is_configured(&self) -> bool {
        !self.input.is_empty() && self.target_scene != NO_SCENE
    }

    fn restart_monitoring(&mut self) {
        self.last_frame_time = None;
        self.last_frame_arrived = Instant::now();
        self.switched = false;
    }

    fn seconds_since_last_frame(&self) -> f32 {
        self.last_frame_arrived.elapsed().as_secs_f32()
    }

    fn switch_to_target_scene(&mut self, api: &UpdateParams) {
        self.switched = true;

        let lost = format!(
            "No frame from {} stream {} for {:.1} s",
            input_name(api, &self.input),
            self.stream,
            self.seconds_since_last_frame()
        );

        if api.set_selected_scene(self.target_scene) && set_live_scene(api, self.target_scene) {
            info!("{lost}, switched to {}", scene_name(api, self.target_scene));
        } else {
            warn!(
                "{lost}, but scene {} could not be made live",
                self.target_scene
            );
        }
    }

    fn input_gui(&mut self, api: &GuiParams) {
        let inputs: Vec<String> = api
            .all_entity_uuids()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .filter(|uuid| {
                api.entity_type_name(uuid)
                    .is_some_and(|type_name| INPUT_ENTITY_TYPES.contains(&type_name.as_str()))
            })
            .collect();

        if inputs.is_empty() {
            api.label("No input entities found.");
            return;
        }

        let names: Vec<String> = inputs.iter().map(|uuid| input_name(api, uuid)).collect();
        let selected = inputs.iter().position(|uuid| *uuid == self.input);

        if let Some(index) = combo_selection(api, "Input to monitor", &names, selected) {
            self.input.clone_from(&inputs[index]);
            api.write_string(INPUT_KEY, &self.input);
            self.restart_monitoring();
        }
    }

    fn target_scene_gui(&mut self, api: &GuiParams) {
        let scenes = api.all_scene_ids();

        if scenes.is_empty() {
            api.label("No scenes found.");
            return;
        }

        let names: Vec<String> = scenes.iter().map(|scene| scene_name(api, *scene)).collect();
        let selected = scenes.iter().position(|scene| *scene == self.target_scene);

        if let Some(index) = combo_selection(api, "Target scene", &names, selected) {
            self.target_scene = scenes[index];
            api.write_int(TARGET_SCENE_KEY, self.target_scene);
        }
    }

    fn status_gui(&self, api: &GuiParams) {
        if !self.is_configured() {
            api.label_colored("Choose an input and a target scene.", YELLOW);
        } else if !self.active {
            api.label_colored("Monitoring inactive.", DIM);
        } else if self.switched {
            api.label_colored("Input lost, switched to the target scene.", RED);
        } else if self.last_frame_time.is_none() {
            api.label_colored(
                &format!("Waiting for the first frame on stream {}.", self.stream),
                YELLOW,
            );
        } else {
            api.label_colored(
                &format!(
                    "Monitoring. Last frame {:.1} s ago.",
                    self.seconds_since_last_frame()
                ),
                GREEN,
            );
        }
    }
}

impl OdenPlugin for SceneChangeOnLostInput {
    fn init(api: &InitParams) -> Self {
        Self {
            input: api.read_string(INPUT_KEY).unwrap_or_default(),
            stream: api.read_int(STREAM_KEY).unwrap_or(1),
            timeout_s: api
                .read_float(TIMEOUT_KEY)
                .unwrap_or(0.2)
                .clamp(MIN_TIMEOUT_S, MAX_TIMEOUT_S),
            target_scene: api.read_int(TARGET_SCENE_KEY).unwrap_or(NO_SCENE),
            active: api.read_int(ACTIVE_KEY).unwrap_or(0) != 0,
            last_frame_time: None,
            last_frame_arrived: Instant::now(),
            switched: false,
        }
    }

    fn shutdown(self, _api: &ShutdownParams) {}

    fn update(&mut self, api: &UpdateParams) {
        if !self.active || !self.is_configured() {
            return;
        }

        let frame_time = api.camera_last_received_frame_time(&self.input, self.stream);

        if frame_time.is_some() && frame_time != self.last_frame_time {
            self.last_frame_time = frame_time;
            self.last_frame_arrived = Instant::now();
            self.switched = false;
        }

        let input_lost =
            self.last_frame_time.is_some() && self.seconds_since_last_frame() > self.timeout_s;

        if input_lost && !self.switched {
            self.switch_to_target_scene(api);
        }
    }

    fn gui(&mut self, api: &GuiParams) {
        self.input_gui(api);

        if api.drag_int("Stream ID", 0.05, 1, 64, "%d", &mut self.stream) {
            api.write_int(STREAM_KEY, self.stream);
            self.restart_monitoring();
        }

        if api.drag_float(
            "Timeout",
            0.01,
            MIN_TIMEOUT_S,
            MAX_TIMEOUT_S,
            "%.2f s",
            1.0,
            &mut self.timeout_s,
        ) {
            api.write_float(TIMEOUT_KEY, self.timeout_s);
        }

        self.target_scene_gui(api);

        if api.checkbox("Active", &mut self.active) {
            api.write_int(ACTIVE_KEY, i32::from(self.active));
            self.restart_monitoring();
        }

        self.status_gui(api);
    }
}

/// Shows a combo box and returns the index of a newly picked item.
fn combo_selection(
    api: &GuiParams,
    label: &str,
    names: &[String],
    selected: Option<usize>,
) -> Option<usize> {
    let mut index = selected
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(-1);
    let names: Vec<&str> = names.iter().map(String::as_str).collect();

    if !api.combo(label, &mut index, &names) {
        return None;
    }

    usize::try_from(index)
        .ok()
        .filter(|&index| Some(index) != selected)
}

fn input_name(api: &impl SceneApi, input: &str) -> String {
    api.entity_name(input)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| input.to_string())
}

fn scene_name(api: &impl SceneApi, scene: i32) -> String {
    api.scene_name(scene)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Scene {scene}"))
}

// Making a scene live is Producer-only, so oden_plugin_rs does not wrap it
fn set_live_scene(api: &UpdateParams, scene: i32) -> bool {
    unsafe {
        (*api.inner)
            .setLiveScene
            .is_some_and(|set_live_scene| set_live_scene(scene))
    }
}

register_plugin!(
    "Scene Change on Lost Input",
    "60fb4221-d82a-494a-8caa-4c1ba787a4e4",
    SceneChangeOnLostInput
);
