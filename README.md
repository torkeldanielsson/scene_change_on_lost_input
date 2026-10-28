# Scene Change on Lost Input

A Voysys VR Producer plugin that switches to a chosen scene when an input stops
delivering frames, for example to cut to a backup camera or a holding slate.

It works with Producer 3.6.1 and 3.7.0.

## Use

1. Put `scene_change_on_lost_input.dll` in the same folder as your project file
   (`.vproj`) and re-open the project. Producer always loads plugins that sit
   next to the project file.
2. In the scene that shows the input, select an entity in the project tree,
   click **Add Child** and pick **Scene Change on Lost Input**.
3. Set it up:
   - **Input to monitor**: a camera or video input (ZCam/Meta, Kandao, Insta360,
     Stitched Video, Equirectangular Video or 2D Video).
   - **Stream ID**: the stream of that input to watch. Streams are numbered
     from 1 (the default).
   - **Timeout**: how long the stream may go without a new frame, 0.1 to 5 s
     (default 0.2 s).
   - **Target scene**: the scene to switch to, such as the backup.
   - **Active**: tick to start monitoring.

The status line under the settings shows what the plugin sees. Monitoring
starts as soon as Producer reports a frame for the stream; if the stream has
already stalled by then, the plugin switches after the timeout. When no new
frame arrives for longer than the timeout, the plugin selects the target scene,
makes it live and writes a line to the log. It does not switch back, and it
switches again only after frames have resumed and stopped once more.

The plugin runs while its scene is live or selected in the editor, so it also
switches the live output when you are only previewing its scene and the input
drops.

## Build

On Windows:

1. Install Rust from <https://rustup.rs>. Accept the offer to install the Visual
   Studio build tools.
2. Install Git from <https://git-scm.com>.
3. In a new terminal:

   ```
   git clone https://github.com/torkeldanielsson/scene_change_on_lost_input.git
   cd scene_change_on_lost_input
   cargo build --release
   ```

The plugin is `target\release\scene_change_on_lost_input.dll`. It needs no
Visual C++ runtime.

## Change it

All the code is in `src/lib.rs`. To try a change, run `cargo build --release`,
close Producer, copy the new DLL over the old one next to the `.vproj` and open
the project again.

The plugin uses [oden_plugin_rs](https://github.com/voysys/oden_plugin_rs)
v52.1, the Rust API for Oden plugins (Producer is built on Oden). Its
[API documentation](https://voysys.github.io/oden_plugin_rs/52.1.0/oden_plugin_rs/)
and [examples](https://github.com/voysys/oden_plugin_rs/tree/v52.1/examples)
show what a plugin can do. The crate follows newer Oden releases than Producer,
so some of its functions are missing in Producer: calling one stops the plugin
with a "too old" error in the log. Making a scene live is the reverse, a
Producer-only call the crate does not wrap; `set_live_scene` in `src/lib.rs`
calls it directly.

If you make a new plugin from this one, give it a new name and entity ID (the
UUID in `register_plugin!`), so Producer can tell the two apart.

## License

MIT, see [LICENSE](LICENSE).
