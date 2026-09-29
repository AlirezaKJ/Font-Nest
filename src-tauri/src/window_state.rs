//! Where the window was when it last closed, and whether that is still somewhere it can open.
//!
//! Putting a window back is not just replaying the numbers. Monitors get unplugged, laptops get
//! undocked, and a resolution change moves the desktop out from under a saved rectangle. A window
//! restored onto a screen that is no longer there is invisible and, with no title bar to grab,
//! unreachable. So the saved rectangle is checked against the monitors that exist right now, and a
//! position that no longer lands on one is dropped while the size is kept.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{
    LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow, Window,
    WindowEvent,
};

/// Sits beside the managed-installation ledger in the application data directory.
const STATE_FILE_NAME: &str = "window-state.json";

/// Bumped when the stored shape changes. A version this build does not know is ignored rather
/// than guessed at, the same way the ledger refuses a schema from the future.
const SCHEMA_VERSION: u32 = 1;

/// How much of the window has to land on a monitor for it to count as reachable. A window is
/// draggable by its title bar, so a strip that size is enough to pull the rest back into view.
const MIN_VISIBLE_WIDTH: i64 = 120;
const MIN_VISIBLE_HEIGHT: i64 = 40;

/// The floor from `tauri.conf.json`, in the logical pixels that file is written in. Everything
/// else here is physical, so this is converted against the window's scale factor before it is
/// compared with anything. `window-state.spec.ts` fails if the two files ever disagree.
pub const MINIMUM_LOGICAL_SIZE: (u32, u32) = (520, 480);

/// Where the window sat, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// A monitor's usable area, in the same physical pixels, with the taskbar already taken off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// What to do with a saved rectangle now that the monitors have had their say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restore {
    /// Still lands on a monitor: put the window back exactly where it was.
    Exact(WindowState),
    /// The monitor is gone or has moved. Keep the size the person chose and let the window
    /// centre itself on a screen that does exist.
    Centred {
        width: u32,
        height: u32,
        maximized: bool,
    },
}

/// Overlap between two rectangles along one axis, in pixels.
fn overlap(start: i64, length: i64, other_start: i64, other_length: i64) -> i64 {
    let end = start + length;
    let other_end = other_start + other_length;
    (end.min(other_end) - start.max(other_start)).max(0)
}

/// Whether enough of `window` lands on `monitor` to grab hold of it.
fn reachable_on(window: Area, monitor: Area) -> bool {
    let width = i64::from(window.width);
    let height = i64::from(window.height);
    // A window smaller than the strip we ask for still counts when all of it is showing.
    let needed_width = MIN_VISIBLE_WIDTH.min(width);
    let needed_height = MIN_VISIBLE_HEIGHT.min(height);

    overlap(
        i64::from(window.x),
        width,
        i64::from(monitor.x),
        i64::from(monitor.width),
    ) >= needed_width
        && overlap(
            i64::from(window.y),
            height,
            i64::from(monitor.y),
            i64::from(monitor.height),
        ) >= needed_height
}

/// Works out how to reopen at `saved` given the monitors attached right now.
///
/// The size is always clamped: never below the window's own minimum, and never larger than the
/// biggest monitor available, so a window saved on a large screen still fits a small one.
#[must_use]
pub fn plan_restore(saved: WindowState, monitors: &[Area], minimum: (u32, u32)) -> Restore {
    let (min_width, min_height) = minimum;
    let widest = monitors.iter().map(|area| area.width).max();
    let tallest = monitors.iter().map(|area| area.height).max();

    let width = saved
        .width
        .clamp(min_width, widest.unwrap_or(saved.width).max(min_width));
    let height = saved
        .height
        .clamp(min_height, tallest.unwrap_or(saved.height).max(min_height));

    let window = Area {
        x: saved.x,
        y: saved.y,
        width,
        height,
    };

    if monitors
        .iter()
        .any(|monitor| reachable_on(window, *monitor))
    {
        Restore::Exact(WindowState {
            x: saved.x,
            y: saved.y,
            width,
            height,
            maximized: saved.maximized,
        })
    } else {
        Restore::Centred {
            width,
            height,
            maximized: saved.maximized,
        }
    }
}

/// The stored document. The version rides alongside the rectangle so a later shape can be told
/// apart from this one instead of being read as a broken copy of it.
#[derive(Serialize, Deserialize)]
struct StoredState {
    version: u32,
    #[serde(flatten)]
    window: WindowState,
}

#[must_use]
pub fn state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(STATE_FILE_NAME)
}

/// Reads the saved rectangle, or `None` when there is nothing usable to read.
///
/// A missing file is the ordinary first-run case. A corrupt or newer file is treated the same
/// way: the window opens where the configuration says, which is always a safe place to be.
#[must_use]
pub fn load(app_data_dir: &Path) -> Option<WindowState> {
    let contents = fs::read_to_string(state_path(app_data_dir)).ok()?;
    let stored: StoredState = serde_json::from_str(&contents).ok()?;
    if stored.version != SCHEMA_VERSION {
        return None;
    }
    if stored.window.width == 0 || stored.window.height == 0 {
        return None;
    }
    Some(stored.window)
}

/// Writes the rectangle through a temporary file, so a crash mid-write cannot leave a half
/// document that the next launch has to puzzle over.
///
/// # Errors
///
/// Returns the underlying error when the directory cannot be created or the file cannot be
/// written or renamed into place.
pub fn save(app_data_dir: &Path, window: WindowState) -> io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let document = serde_json::to_string_pretty(&StoredState {
        version: SCHEMA_VERSION,
        window,
    })
    .map_err(io::Error::other)?;

    let path = state_path(app_data_dir);
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, document)?;
    fs::rename(&temporary, &path)
}

/// The last rectangle the window had while it was neither maximized nor minimized.
///
/// A maximized window reports the whole screen, and a minimized one reports a position far off
/// the desktop, so neither is worth writing down. Remembering the ordinary rectangle is what lets
/// a window reopen maximized and still have somewhere sensible to go when it is restored down.
#[derive(Default)]
pub struct Tracker(Mutex<Option<WindowState>>);

impl Tracker {
    pub fn record(&self, window: WindowState) {
        if let Ok(mut held) = self.0.lock() {
            *held = Some(window);
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Option<WindowState> {
        self.0.lock().ok().and_then(|held| *held)
    }
}

/// The label of the window whose rectangle is worth remembering. Any other window `FontNest`
/// opens later is incidental and should not overwrite it.
const MAIN_WINDOW: &str = "main";

/// The rectangle the window occupies right now, or `None` while it is maximized, minimized, or
/// otherwise not reporting a rectangle worth keeping.
fn current<R: Runtime>(window: &Window<R>) -> Option<WindowState> {
    if matches!(window.is_minimized(), Ok(true)) || matches!(window.is_maximized(), Ok(true)) {
        return None;
    }

    // The inner size on purpose: `set_size` sets the inner size, so measuring the outer one
    // here would hand back a rectangle that shrinks by the window frame on every launch.
    let position = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    Some(WindowState {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: false,
    })
}

/// Puts the window back where it was left.
///
/// Called before the window is shown, so a restored position is never a visible jump. Every
/// failure here is survivable: the window simply opens where `tauri.conf.json` says.
pub fn restore<R: Runtime>(window: &WebviewWindow<R>, app_data_dir: &Path) {
    let Some(saved) = load(app_data_dir) else {
        return;
    };

    let monitors = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let area = monitor.work_area();
            Area {
                x: area.position.x,
                y: area.position.y,
                width: area.size.width,
                height: area.size.height,
            }
        })
        .collect::<Vec<_>>();

    // `tauri.conf.json` states the minimum in logical pixels; the saved rectangle and the
    // monitors are physical, so the floor is converted before the two are compared.
    let scale = window.scale_factor().unwrap_or(1.0);
    let minimum: PhysicalSize<u32> =
        LogicalSize::new(MINIMUM_LOGICAL_SIZE.0, MINIMUM_LOGICAL_SIZE.1).to_physical(scale);

    let (placed, ordinary) = match plan_restore(saved, &monitors, (minimum.width, minimum.height)) {
        Restore::Exact(state) => {
            let sized = window.set_size(PhysicalSize::new(state.width, state.height));
            let moved = window.set_position(PhysicalPosition::new(state.x, state.y));
            if let Err(error) = sized.and(moved) {
                log::warn!("FontNest could not restore the window rectangle: {error}");
            }
            (state.maximized, state)
        }
        Restore::Centred {
            width,
            height,
            maximized,
        } => {
            log::info!(
                "The saved window position is not on any monitor attached now, so FontNest is centring the window instead."
            );
            if let Err(error) = window
                .set_size(PhysicalSize::new(width, height))
                .and_then(|()| window.center())
            {
                log::warn!("FontNest could not centre the window: {error}");
            }
            (
                maximized,
                WindowState {
                    x: 0,
                    y: 0,
                    width,
                    height,
                    maximized,
                },
            )
        }
    };

    // Seed the tracker from the restored rectangle, so closing without touching the window
    // writes back what was read rather than nothing.
    window.state::<Tracker>().record(WindowState {
        maximized: false,
        ..ordinary
    });

    if placed && let Err(error) = window.maximize() {
        log::warn!("FontNest could not restore the maximized window: {error}");
    }
}

/// Follows the window around, and writes the rectangle down when it closes.
pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW {
        return;
    }

    match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            if let Some(state) = current(window) {
                window.state::<Tracker>().record(state);
            }
        }
        WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => persist(window),
        _ => {}
    }
}

/// Writes the last ordinary rectangle, with whether the window was maximized over the top of it.
fn persist<R: Runtime>(window: &Window<R>) {
    let Some(ordinary) = window
        .state::<Tracker>()
        .snapshot()
        .or_else(|| current(window))
    else {
        return;
    };

    let state = WindowState {
        maximized: matches!(window.is_maximized(), Ok(true)),
        ..ordinary
    };

    let Ok(app_data_dir) = window.path().app_data_dir() else {
        return;
    };

    if let Err(error) = save(&app_data_dir, state) {
        log::warn!("FontNest could not remember the window position: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Area, MINIMUM_LOGICAL_SIZE, Restore, SCHEMA_VERSION, Tracker, WindowState, load,
        plan_restore, save, state_path,
    };
    use std::fs;
    use tempfile::tempdir;

    /// A laptop screen at the origin, and a second monitor to its right.
    fn two_monitors() -> Vec<Area> {
        vec![
            Area {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
            Area {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1400,
            },
        ]
    }

    fn state(x: i32, y: i32, width: u32, height: u32) -> WindowState {
        WindowState {
            x,
            y,
            width,
            height,
            maximized: false,
        }
    }

    #[test]
    fn a_window_inside_a_monitor_goes_back_where_it_was() {
        let saved = state(200, 120, 1180, 760);

        assert_eq!(
            plan_restore(saved, &two_monitors(), MINIMUM_LOGICAL_SIZE),
            Restore::Exact(saved)
        );
    }

    #[test]
    fn a_window_on_the_second_monitor_goes_back_where_it_was() {
        let saved = state(2400, 300, 1180, 760);

        assert_eq!(
            plan_restore(saved, &two_monitors(), MINIMUM_LOGICAL_SIZE),
            Restore::Exact(saved)
        );
    }

    #[test]
    fn a_window_straddling_two_monitors_goes_back_where_it_was() {
        let saved = state(1500, 200, 1180, 760);

        assert_eq!(
            plan_restore(saved, &two_monitors(), MINIMUM_LOGICAL_SIZE),
            Restore::Exact(saved)
        );
    }

    #[test]
    fn a_window_on_a_monitor_that_is_gone_keeps_its_size_and_is_centred() {
        let saved = state(2400, 300, 1180, 760);
        let laptop_only = &two_monitors()[..1];

        assert_eq!(
            plan_restore(saved, laptop_only, MINIMUM_LOGICAL_SIZE),
            Restore::Centred {
                width: 1180,
                height: 760,
                maximized: false,
            }
        );
    }

    #[test]
    fn a_window_with_only_a_sliver_showing_is_centred() {
        // Ten pixels of the left edge are on the laptop screen: not enough to grab.
        let saved = state(1910, 400, 1180, 760);
        let laptop_only = &two_monitors()[..1];

        assert!(matches!(
            plan_restore(saved, laptop_only, MINIMUM_LOGICAL_SIZE),
            Restore::Centred { .. }
        ));
    }

    #[test]
    fn a_window_above_the_desktop_is_centred() {
        let saved = state(200, -900, 1180, 760);

        assert!(matches!(
            plan_restore(saved, &two_monitors(), MINIMUM_LOGICAL_SIZE),
            Restore::Centred { .. }
        ));
    }

    #[test]
    fn a_window_saved_on_a_larger_screen_is_cut_down_to_fit_this_one() {
        let saved = state(0, 0, 2500, 1300);
        let laptop_only = &two_monitors()[..1];

        assert_eq!(
            plan_restore(saved, laptop_only, MINIMUM_LOGICAL_SIZE),
            Restore::Exact(state(0, 0, 1920, 1040))
        );
    }

    #[test]
    fn a_size_below_the_window_minimum_is_raised_to_it() {
        let saved = state(100, 100, 120, 90);

        assert_eq!(
            plan_restore(saved, &two_monitors(), MINIMUM_LOGICAL_SIZE),
            Restore::Exact(state(
                100,
                100,
                MINIMUM_LOGICAL_SIZE.0,
                MINIMUM_LOGICAL_SIZE.1
            ))
        );
    }

    #[test]
    fn maximized_survives_the_journey() {
        let saved = WindowState {
            maximized: true,
            ..state(200, 120, 1180, 760)
        };

        assert_eq!(
            plan_restore(saved, &two_monitors(), MINIMUM_LOGICAL_SIZE),
            Restore::Exact(saved)
        );
    }

    #[test]
    fn with_no_monitors_to_check_against_the_window_is_centred() {
        let saved = state(200, 120, 1180, 760);

        assert_eq!(
            plan_restore(saved, &[], MINIMUM_LOGICAL_SIZE),
            Restore::Centred {
                width: 1180,
                height: 760,
                maximized: false,
            }
        );
    }

    #[test]
    fn a_saved_rectangle_reads_back_as_it_was_written() {
        let directory = tempdir().expect("a temporary directory");
        let saved = WindowState {
            maximized: true,
            ..state(-40, 90, 1400, 900)
        };

        save(directory.path(), saved).expect("the state writes");

        assert_eq!(load(directory.path()), Some(saved));
    }

    #[test]
    fn nothing_saved_yet_reads_as_nothing() {
        let directory = tempdir().expect("a temporary directory");

        assert_eq!(load(directory.path()), None);
    }

    #[test]
    fn a_version_this_build_does_not_know_is_left_alone() {
        let directory = tempdir().expect("a temporary directory");
        let document = format!(
            r#"{{"version":{},"x":10,"y":10,"width":1180,"height":760,"maximized":false}}"#,
            SCHEMA_VERSION + 1
        );
        fs::write(state_path(directory.path()), &document).expect("the fixture writes");

        assert_eq!(load(directory.path()), None);
        assert_eq!(
            fs::read_to_string(state_path(directory.path())).expect("the file is still there"),
            document
        );
    }

    #[test]
    fn a_half_written_document_reads_as_nothing() {
        let directory = tempdir().expect("a temporary directory");
        fs::write(state_path(directory.path()), r#"{"version":1,"x":10,"#)
            .expect("the fixture writes");

        assert_eq!(load(directory.path()), None);
    }

    #[test]
    fn a_rectangle_with_no_area_reads_as_nothing() {
        let directory = tempdir().expect("a temporary directory");
        fs::write(
            state_path(directory.path()),
            r#"{"version":1,"x":10,"y":10,"width":0,"height":0,"maximized":false}"#,
        )
        .expect("the fixture writes");

        assert_eq!(load(directory.path()), None);
    }

    #[test]
    fn saving_twice_leaves_one_document_and_no_temporary_file() {
        let directory = tempdir().expect("a temporary directory");

        save(directory.path(), state(10, 10, 1180, 760)).expect("the first write");
        save(directory.path(), state(20, 20, 1200, 800)).expect("the second write");

        let left = fs::read_dir(directory.path())
            .expect("the directory lists")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(left, vec!["window-state.json".to_owned()]);
        assert_eq!(load(directory.path()), Some(state(20, 20, 1200, 800)));
    }

    #[test]
    fn the_tracker_holds_the_last_ordinary_rectangle() {
        let tracker = Tracker::default();
        assert_eq!(tracker.snapshot(), None);

        tracker.record(state(10, 10, 1180, 760));
        tracker.record(state(30, 40, 1200, 800));

        assert_eq!(tracker.snapshot(), Some(state(30, 40, 1200, 800)));
    }
}
