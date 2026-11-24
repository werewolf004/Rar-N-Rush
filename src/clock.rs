//! Local date and time, for the log and the names of backup and export files.

use time::OffsetDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

/// `2026-11-24 14:05:09`: lines of the log file.
const LOG_LINE: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
/// `14:05:09`: lines of the log panel.
const TIME_OF_DAY: &[BorrowedFormatItem<'_>] = format_description!("[hour]:[minute]:[second]");
/// `20261124-140509`: file names (backups, exported logs).
const STAMP: &[BorrowedFormatItem<'_>] = format_description!("[year][month][day]-[hour][minute][second]");

/// One reading of the clock, so that several formats show the same instant.
#[derive(Debug, Clone, Copy)]
pub struct Now(OffsetDateTime);

/// Current local time; UTC if the local offset cannot be read.
pub fn now() -> Now {
    Now(OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()))
}

impl Now {
    /// `2026-11-24 14:05:09`.
    pub fn log_line(self) -> String {
        self.format(LOG_LINE)
    }

    /// `14:05:09`.
    pub fn time_of_day(self) -> String {
        self.format(TIME_OF_DAY)
    }

    /// `20261124-140509`, for file names.
    pub fn stamp(self) -> String {
        self.format(STAMP)
    }

    fn format(self, description: &[BorrowedFormatItem<'_>]) -> String {
        // Only fails for components the date does not have; these formats use
        // none, so this never returns the empty string in practice.
        self.0.format(description).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn formats_are_zero_padded_and_24_hour() {
        let at = Now(datetime!(2026-03-04 07:08:09 +01:00));
        assert_eq!(at.log_line(), "2026-03-04 07:08:09");
        assert_eq!(at.time_of_day(), "07:08:09");
        assert_eq!(at.stamp(), "20260304-070809");
        let evening = Now(datetime!(2026-12-31 23:59:58 UTC));
        assert_eq!(evening.time_of_day(), "23:59:58");
    }

    #[test]
    fn now_reads_the_clock() {
        let line = now().log_line();
        assert_eq!(line.len(), 19, "{line}");
        assert!(line.starts_with("20"), "{line}");
    }
}
