#![allow(deprecated)]

use chrono::{
    Date, DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc,
};
use specta::Type;

#[test]
fn chrono() {
    #[derive(Type)]
    #[specta(collect = false)]
    #[allow(dead_code)]
    struct Chrono {
        date: (NaiveDate, Date<Utc>, Date<Local>, Date<FixedOffset>),
        time: NaiveTime,
        date_time: (
            NaiveDateTime,
            DateTime<Utc>,
            DateTime<Local>,
            DateTime<FixedOffset>,
        ),
        duration: Duration,
    }

    insta::assert_snapshot!(crate::ts::inline::<Chrono>(&Default::default()).unwrap(), @"{ date: [string, string, string, string]; time: string; date_time: [string, string, string, string]; duration: string }");
}
