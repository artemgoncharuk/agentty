//! Mouse support E2E tests: wheel scrolling over the session transcript,
//! clicks on tabs, list rows, and dropdown options, and the `Mouse Support`
//! settings switch.
//!
//! Pointer input is injected as raw SGR mouse sequences (`ESC [ < 64 ; col ;
//! row M` for wheel-up, `65` for wheel-down, `0` with `M`/`m` for a left
//! press/release; coordinates are 1-based) written straight to the PTY, the
//! same way the CSI-u and bracketed-paste tests inject their escape sequences.
//!
//! The semantic PTY run and the VHS recording share one environment, so each
//! scenario seeds its starting tab instead of pressing `Tab` (which persists
//! the active tab) and leaves every persisted setting as it found it.
//!
//! The wheel and click scenarios have no feature page or GIF: VHS types every
//! character as a separate browser key event, so the leading `ESC` reaches
//! crossterm alone and is parsed as a plain `Esc` key instead of a mouse
//! event. Only the PTY executor can deliver the SGR sequence atomically.

use agentty::app::Tab;
use agentty::domain::session_message::SessionMessageKind;
use testty::assertion;
use testty::region::Region;

use crate::common;
use crate::common::{BuilderEnv, FeatureTest, SessionSeed};
use crate::test_support::persist_active_tab_for_test;

type E2eResult = Result<(), Box<dyn std::error::Error>>;

/// Stable id for the seeded session with a transcript taller than the view.
const SCROLL_SESSION_ID: &str = "mouse-scroll-0001";

/// Number of transcript paragraphs seeded so the output overflows the panel.
const TRANSCRIPT_PARAGRAPH_COUNT: usize = 40;

/// One SGR wheel-up notch over the middle of the transcript panel.
const WHEEL_UP_OVER_TRANSCRIPT: &str = "\x1b[<64;40;10M";

/// One SGR wheel-down notch over the middle of the transcript panel.
const WHEEL_DOWN_OVER_TRANSCRIPT: &str = "\x1b[<65;40;10M";

/// Returns the SGR press-and-release sequence for a left click at 1-based
/// terminal coordinates.
fn left_click(column: u16, row: u16) -> String {
    format!("\x1b[<0;{column};{row}M\x1b[<0;{column};{row}m")
}

/// Header line of the list pages, 1-based.
const TAB_BAR_ROW: u16 = 3;

/// A column inside the ` Sessions ` header label. The seeded fixture is not a
/// Git repository, so the header reads `Project: None` and the labels sit at
/// fixed columns.
const SESSIONS_TAB_COLUMN: u16 = 32;

/// A column inside the ` Settings ` header label (see `SESSIONS_TAB_COLUMN`).
const SETTINGS_TAB_COLUMN: u16 = 43;

/// First and second session rows of the `ACTIVE` group on the Sessions tab,
/// 1-based: the table border, header, and group label sit above them.
const FIRST_SESSION_ROW: u16 = 8;
const SECOND_SESSION_ROW: u16 = 9;

/// The `Mouse Support` row on the Settings tab, 1-based.
const MOUSE_SUPPORT_ROW: u16 = 9;

/// First option of the dropdown opened from the `Mouse Support` row, 1-based:
/// the popup opens one row below its setting with a border and one padding
/// line above the options.
const MOUSE_SUPPORT_FIRST_OPTION_ROW: u16 = 12;

/// A column inside the settings dropdown popup on an 80-column terminal.
const SETTINGS_DROPDOWN_COLUMN: u16 = 60;

/// Seeds one review-ready session whose transcript is taller than the view and
/// starts the app on the Sessions tab.
///
/// Paragraph labels avoid spaces because testty's text search skips blank
/// cells the terminal never repainted.
async fn seed_session_with_long_transcript(env: &BuilderEnv) -> E2eResult {
    common::seed_session(
        env,
        SessionSeed::regular(SCROLL_SESSION_ID, "claude-opus-5", "main", "Review")
            .with_title("Mouse wheel scrolling"),
    )
    .await?;

    let transcript = (1..=TRANSCRIPT_PARAGRAPH_COUNT)
        .map(|index| format!("transcript_line_{index:02}"))
        .collect::<Vec<_>>()
        .join("\n\n");

    (async {
        let database = common::open_database(env).await?;
        database
            .sessions()
            .append_session_message(
                SCROLL_SESSION_ID,
                SessionMessageKind::AssistantAnswer,
                &transcript,
            )
            .await?;
        persist_active_tab_for_test(&database, Tab::Sessions).await
    })
    .await?;

    std::fs::create_dir_all(env.agentty_root.join("wt").join(&SCROLL_SESSION_ID[..8]))?;

    Ok(())
}

/// Verify that the mouse wheel scrolls the session transcript and that
/// scrolling back down resumes following the newest output.
#[tokio::test]
async fn mouse_wheel_scrolls_session_output() {
    // Arrange, Act, Assert
    FeatureTest::new("mouse_wheel_scroll")
        .setup(|env| Box::pin(async move { seed_session_with_long_transcript(env).await }))
        .run(
            |scenario| {
                scenario
                    .compose(&common::wait_for_agentty_startup())
                    .compose(&common::open_selected_session_view())
                    .wait_for_text("transcript_line_40", 5000)
                    .viewing_pause_ms(1500)
                    .capture_labeled("tail", "Session view following the newest output")
                    .write_text(WHEEL_UP_OVER_TRANSCRIPT)
                    .write_text(WHEEL_UP_OVER_TRANSCRIPT)
                    .write_text(WHEEL_UP_OVER_TRANSCRIPT)
                    .wait_for_stable_frame(200, 3000)
                    .viewing_pause_ms(1500)
                    .capture_labeled("scrolled_up", "Transcript scrolled up with the wheel")
                    .write_text(WHEEL_DOWN_OVER_TRANSCRIPT)
                    .write_text(WHEEL_DOWN_OVER_TRANSCRIPT)
                    .write_text(WHEEL_DOWN_OVER_TRANSCRIPT)
                    .write_text(WHEEL_DOWN_OVER_TRANSCRIPT)
                    .wait_for_text("transcript_line_40", 3000)
                    .viewing_pause_ms(1500)
                    .capture_labeled("back_at_tail", "Wheel-down returned the view to the tail")
            },
            |frame, report| {
                Box::pin(async move {
                    assert_eq!(
                        report.captures.len(),
                        3,
                        "Expected 3 captures (tail, scrolled_up, back_at_tail)"
                    );

                    let tail_frame = common::frame_from_capture(&report.captures[0]);
                    let tail_full = Region::full(tail_frame.cols(), tail_frame.rows());
                    assertion::assert_text_in_region(&tail_frame, "transcript_line_40", &tail_full);
                    assertion::assert_text_in_region(&tail_frame, "█", &tail_full);

                    let scrolled_frame = common::frame_from_capture(&report.captures[1]);
                    let scrolled_full = Region::full(scrolled_frame.cols(), scrolled_frame.rows());
                    assertion::assert_not_visible(&scrolled_frame, "transcript_line_40");
                    assertion::assert_text_in_region(
                        &scrolled_frame,
                        "transcript_line_3",
                        &scrolled_full,
                    );

                    let full = Region::full(frame.cols(), frame.rows());
                    assertion::assert_text_in_region(frame, "transcript_line_40", &full);
                })
            },
        )
        .await
        .expect("feature test failed");
}

/// Starts the app on the Settings tab so the scenario never presses `Tab`.
async fn seed_settings_tab(env: &BuilderEnv) -> E2eResult {
    (async {
        let database = common::open_database(env).await?;
        persist_active_tab_for_test(&database, Tab::Settings).await
    })
    .await?;

    Ok(())
}

/// Verify that the `Mouse Support` switch lives in the global settings section
/// and can be turned off and back on from its dropdown.
///
/// The switch is restored to `Enabled` at the end so the VHS recording starts
/// from the same persisted state as the semantic run.
#[tokio::test]
async fn settings_mouse_support_switch() {
    // Arrange, Act, Assert
    FeatureTest::new("settings_mouse_support")
        .setup(|env| Box::pin(async move { seed_settings_tab(env).await }))
        .zola(
            "Mouse support switch",
            "Turn terminal mouse capture off and back on from the global settings.",
            157,
        )
        .run(
            |scenario| {
                scenario
                    .compose(&common::wait_for_agentty_startup())
                    .wait_for_text("Mouse Support", 5000)
                    .viewing_pause_ms(1500)
                    .capture_labeled("enabled", "Mouse Support enabled by default")
                    .press_key("j")
                    .wait_for_stable_frame(200, 3000)
                    .press_key("j")
                    .wait_for_stable_frame(200, 3000)
                    .press_key("j")
                    .wait_for_stable_frame(200, 3000)
                    .press_key("Enter")
                    .wait_for_text("Select setting value", 3000)
                    .viewing_pause_ms(1500)
                    .capture_labeled("dropdown", "Mouse Support dropdown")
                    .press_key("k")
                    .wait_for_stable_frame(200, 3000)
                    .press_key("Enter")
                    .wait_for_stable_frame(200, 3000)
                    .viewing_pause_ms(2000)
                    .capture_labeled("disabled", "Mouse Support turned off")
                    .press_key("Enter")
                    .wait_for_text("Select setting value", 3000)
                    .press_key("j")
                    .wait_for_stable_frame(200, 3000)
                    .press_key("Enter")
                    .wait_for_stable_frame(200, 3000)
                    .viewing_pause_ms(1500)
                    .capture_labeled("restored", "Mouse Support turned back on")
            },
            |frame, report| {
                Box::pin(async move {
                    assert_eq!(
                        report.captures.len(),
                        4,
                        "Expected 4 captures (enabled, dropdown, disabled, restored)"
                    );

                    let enabled_frame = common::frame_from_capture(&report.captures[0]);
                    let enabled_full = Region::full(enabled_frame.cols(), enabled_frame.rows());
                    assertion::assert_text_in_region(
                        &enabled_frame,
                        "Global settings",
                        &enabled_full,
                    );
                    assertion::assert_text_in_region(
                        &enabled_frame,
                        "Mouse Support",
                        &enabled_full,
                    );
                    assertion::assert_match_count(&enabled_frame, "Enabled", 2);

                    let disabled_frame = common::frame_from_capture(&report.captures[2]);
                    let disabled_full = Region::full(disabled_frame.cols(), disabled_frame.rows());
                    assertion::assert_text_in_region(
                        &disabled_frame,
                        "Mouse Support",
                        &disabled_full,
                    );
                    assertion::assert_match_count(&disabled_frame, "Enabled", 1);
                    assertion::assert_match_count(&disabled_frame, "Disabled", 2);

                    let full = Region::full(frame.cols(), frame.rows());
                    assertion::assert_text_in_region(frame, "Mouse Support", &full);
                    assertion::assert_match_count(frame, "Enabled", 2);
                    assertion::assert_match_count(frame, "Disabled", 1);
                })
            },
        )
        .await
        .expect("feature test failed");
}

/// Seeds two review-ready sessions and starts on the Sessions tab.
async fn seed_two_sessions_on_sessions_tab(env: &BuilderEnv) -> E2eResult {
    common::seed_session(
        env,
        SessionSeed::regular("mouse-click-0001", "claude-opus-5", "main", "Review")
            .with_title("First clickable session"),
    )
    .await?;
    common::seed_session(
        env,
        SessionSeed::regular("mouse-click-0002", "claude-opus-5", "main", "Review")
            .with_title("Second clickable session"),
    )
    .await?;
    for session_id in ["mouse-click-0001", "mouse-click-0002"] {
        std::fs::create_dir_all(env.agentty_root.join("wt").join(&session_id[..8]))?;
    }

    (async {
        let database = common::open_database(env).await?;
        persist_active_tab_for_test(&database, Tab::Sessions).await
    })
    .await?;

    Ok(())
}

/// Verify that clicking header labels switches tabs, clicking a session row
/// selects it, and clicking the selected row opens the session like `Enter`.
#[tokio::test]
async fn mouse_click_switches_tabs_and_opens_selected_session() {
    // Arrange, Act, Assert
    FeatureTest::new("mouse_click_session_list")
        .setup(|env| Box::pin(async move { seed_two_sessions_on_sessions_tab(env).await }))
        .run(
            |scenario| {
                scenario
                    .compose(&common::wait_for_agentty_startup())
                    .wait_for_text("new session", 5000)
                    .write_text(left_click(SETTINGS_TAB_COLUMN, TAB_BAR_ROW))
                    .wait_for_text("Default Smart Model", 3000)
                    .capture_labeled("settings_tab", "Settings tab opened by clicking its label")
                    .write_text(left_click(SESSIONS_TAB_COLUMN, TAB_BAR_ROW))
                    .wait_for_text("new session", 3000)
                    .write_text(left_click(10, SECOND_SESSION_ROW))
                    .wait_for_stable_frame(200, 3000)
                    .capture_labeled("second_row_selected", "Second session row selected")
                    .write_text(left_click(10, SECOND_SESSION_ROW))
                    .wait_for_text("q: back", 3000)
            },
            |frame, report| {
                Box::pin(async move {
                    assert_eq!(report.captures.len(), 2);

                    let settings_frame = common::frame_from_capture(&report.captures[0]);
                    let settings_full = Region::full(settings_frame.cols(), settings_frame.rows());
                    assertion::assert_text_in_region(
                        &settings_frame,
                        "Global settings",
                        &settings_full,
                    );

                    // Row selection paints only a background, which `NO_COLOR`
                    // hides, so the opened session proves which row the clicks
                    // selected: it must be the title painted on the clicked
                    // row.
                    let selected_frame = common::frame_from_capture(&report.captures[1]);
                    let first_row_text = selected_frame.row_text(FIRST_SESSION_ROW - 1);
                    let second_row_text = selected_frame.row_text(SECOND_SESSION_ROW - 1);
                    assert!(
                        first_row_text.contains("clickable session")
                            && second_row_text.contains("clickable session"),
                        "both seeded sessions should occupy the expected rows: {first_row_text} / \
                         {second_row_text}"
                    );
                    let clicked_title = if second_row_text.contains("Second clickable session") {
                        "Second clickable session"
                    } else {
                        "First clickable session"
                    };
                    assertion::assert_not_visible(&selected_frame, "q: back");

                    let full = Region::full(frame.cols(), frame.rows());
                    assertion::assert_text_in_region(frame, "q: back", &full);
                    assertion::assert_text_in_region(frame, clicked_title, &full);
                })
            },
        )
        .await
        .expect("feature test failed");
}

/// Verify that clicking a settings row selects it, clicking it again opens
/// its dropdown, and clicking a dropdown option twice selects and confirms it.
#[tokio::test]
async fn mouse_click_selects_settings_rows_and_dropdown_options() {
    // Arrange, Act, Assert
    FeatureTest::new("mouse_click_settings")
        .setup(|env| Box::pin(async move { seed_settings_tab(env).await }))
        .run(
            |scenario| {
                scenario
                    .compose(&common::wait_for_agentty_startup())
                    .wait_for_text("Mouse Support", 5000)
                    .write_text(left_click(10, MOUSE_SUPPORT_ROW))
                    .wait_for_stable_frame(200, 3000)
                    .capture_labeled("row_selected", "Mouse Support row selected by click")
                    .write_text(left_click(10, MOUSE_SUPPORT_ROW))
                    .wait_for_text("Select setting value", 3000)
                    .capture_labeled("dropdown", "Dropdown opened by clicking the selected row")
                    .write_text(left_click(
                        SETTINGS_DROPDOWN_COLUMN,
                        MOUSE_SUPPORT_FIRST_OPTION_ROW,
                    ))
                    .wait_for_stable_frame(200, 3000)
                    .capture_labeled("option_selected", "Disabled option selected by click")
                    .write_text(left_click(
                        SETTINGS_DROPDOWN_COLUMN,
                        MOUSE_SUPPORT_FIRST_OPTION_ROW,
                    ))
                    .wait_for_stable_frame(200, 3000)
            },
            |frame, report| {
                Box::pin(async move {
                    assert_eq!(report.captures.len(), 3);

                    // Row selection paints only a background, which `NO_COLOR`
                    // hides; the dropdown that opens on the second click proves
                    // the first click selected the `Mouse Support` row.
                    let row_frame = common::frame_from_capture(&report.captures[0]);
                    assertion::assert_not_visible(&row_frame, "Select setting value");

                    let dropdown_frame = common::frame_from_capture(&report.captures[1]);
                    let dropdown_full = Region::full(dropdown_frame.cols(), dropdown_frame.rows());
                    assertion::assert_text_in_region(
                        &dropdown_frame,
                        "Select setting value",
                        &dropdown_full,
                    );
                    assertion::assert_text_in_region(&dropdown_frame, "> Enabled", &dropdown_full);

                    let option_frame = common::frame_from_capture(&report.captures[2]);
                    let option_full = Region::full(option_frame.cols(), option_frame.rows());
                    assertion::assert_text_in_region(&option_frame, "> Disabled", &option_full);

                    let full = Region::full(frame.cols(), frame.rows());
                    assertion::assert_not_visible(frame, "Select setting value");
                    assertion::assert_text_in_region(frame, "Mouse Support", &full);
                    assertion::assert_match_count(frame, "Disabled", 2);
                })
            },
        )
        .await
        .expect("feature test failed");
}
