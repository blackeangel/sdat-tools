use indicatif::{ProgressBar, ProgressStyle};

pub fn progress_bar(len: u64) -> ProgressBar {
    ProgressBar::new(len).with_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("#."),
    )
}
