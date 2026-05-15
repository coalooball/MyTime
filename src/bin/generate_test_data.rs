use chrono::{Duration, Local, NaiveDateTime};
use rand::seq::SliceRandom;
use rand::Rng;
use rusqlite::{params, Connection};

const DB_FILE: &str = "time_tracker.db";

struct ActivityGroup {
    category: &'static str,
    activities: &'static [&'static str],
}

const ACTIVITY_GROUPS: &[ActivityGroup] = &[
    ActivityGroup {
        category: "工作",
        activities: &[
            "编写代码",
            "开会",
            "项目规划",
            "代码审查",
            "修复 bug",
            "系统维护",
        ],
    },
    ActivityGroup {
        category: "学习",
        activities: &[
            "阅读技术书籍",
            "在线课程学习",
            "编程练习",
            "算法学习",
            "英语学习",
        ],
    },
    ActivityGroup {
        category: "运动",
        activities: &["跑步", "健身", "游泳", "瑜伽", "篮球"],
    },
    ActivityGroup {
        category: "休息",
        activities: &["午休", "小憩", "冥想", "散步"],
    },
    ActivityGroup {
        category: "娱乐",
        activities: &["看电影", "玩游戏", "听音乐", "阅读小说", "刷视频"],
    },
    ActivityGroup {
        category: "其他",
        activities: &["购物", "整理房间", "做饭", "社交活动"],
    },
];

fn main() -> rusqlite::Result<()> {
    let mut days = 30_i64;
    let mut clear_existing = true;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-d" | "--days" => {
                if let Some(value) = args.next() {
                    days = value.parse().unwrap_or(days);
                }
            }
            "--no-clear" => clear_existing = false,
            _ => {}
        }
    }

    let conn = Connection::open(DB_FILE)?;
    init_db(&conn)?;

    if clear_existing {
        conn.execute("DELETE FROM time_entry", [])?;
        println!("已清除现有数据");
    }

    let count = generate(&conn, days)?;
    let start = Local::now().date_naive() - Duration::days(days);
    let end = Local::now().date_naive();
    println!("成功生成 {count} 条测试数据");
    println!(
        "数据范围: {} 到 {}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    );
    Ok(())
}

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS time_entry (
            id INTEGER NOT NULL PRIMARY KEY,
            start_time DATETIME NOT NULL,
            end_time DATETIME NOT NULL,
            activity VARCHAR(200) NOT NULL,
            category VARCHAR(50) NOT NULL,
            description TEXT
        );
        ",
    )
}

fn generate(conn: &Connection, days: i64) -> rusqlite::Result<usize> {
    let mut rng = rand::thread_rng();
    let mut current_date = Local::now().date_naive() - Duration::days(days);
    let end_date = Local::now().date_naive();
    let mut total = 0;

    while current_date <= end_date {
        let mut last_end: NaiveDateTime = current_date
            .and_hms_opt(9, 0, 0)
            .expect("valid fixed start time");
        let entries = rng.gen_range(2..=5);

        for _ in 0..entries {
            let group = ACTIVITY_GROUPS.choose(&mut rng).expect("activity groups");
            let activity = group.activities.choose(&mut rng).expect("activities");
            let start_time = last_end + Duration::minutes(rng.gen_range(30..=120));
            let end_time = start_time + Duration::minutes(rng.gen_range(30..=180));
            let description = format!("进行{}活动", activity);

            conn.execute(
                "
                INSERT INTO time_entry (start_time, end_time, activity, category, description)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ",
                params![start_time, end_time, activity, group.category, description],
            )?;
            total += 1;
            last_end = end_time;
        }

        current_date += Duration::days(1);
    }

    Ok(total)
}
