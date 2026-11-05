use super::super::database::{Database, half, string, word};
use super::error;
use baken_export::pdb::fixed::{CATEGORY_ROWS, COLORS, COLUMNS, SORT_ROWS};
use rusqlite::{Connection, params, params_from_iter, types::Value};

pub(super) fn insert(db: &Connection, legacy: &Database) -> Result<(), String> {
    for row in legacy.rows(0)? {
        row.get(..94).ok_or("Truncated device track")?;
        let text = |i: usize| string(row, half(row, 0x5e + i * 2)? as usize).map(Value::Text);
        let number = |offset| word(row, offset).map(|v| Value::Integer(v.into()));
        let small = |offset| half(row, offset).map(|v| Value::Integer(v.into()));
        let id = |offset| {
            word(row, offset).map(|v| {
                if v == 0 {
                    Value::Null
                } else {
                    Value::Integer(v.into())
                }
            })
        };
        let values = [
            number(72)?,
            text(17)?,
            Value::Null,
            text(12)?,
            number(56)?,
            small(84)?,
            number(52)?,
            small(76)?,
            id(68)?,
            id(44)?,
            id(36)?,
            id(12)?,
            0.into(),
            id(64)?,
            id(60)?,
            id(40)?,
            id(32)?,
            i64::from(row[88]).into(),
            id(28)?,
            text(16)?,
            i64::from(row[89]).into(),
            small(80)?,
            text(11)?,
            text(10)?,
            text(10)?,
            text(20)?,
            text(19)?,
            number(16)?,
            small(90)?,
            number(48)?,
            small(82)?,
            number(8)?,
            Value::Text(String::new()),
            small(78)?,
            1.into(),
            1.into(),
            Value::Text(String::new()),
            number(24)?,
            number(72)?,
            text(14)?,
            small(86)?,
            number(4)?,
            0.into(),
            Value::Null,
            Value::Null,
            Value::Null,
        ];
        db.execute(
            &format!(
                "INSERT INTO content VALUES ({})",
                vec!["?"; values.len()].join(",")
            ),
            params_from_iter(values),
        )
        .map_err(error)?;
    }
    for (kind, table, offset) in [
        (1, "genre", 4),
        (4, "label", 4),
        (5, "key", 8),
        (13, "image", 4),
    ] {
        for row in legacy.rows(kind)? {
            db.execute(
                &format!("INSERT INTO {table} VALUES (?,?)"),
                params![word(row, 0)?, string(row, offset)?],
            )
            .map_err(error)?;
        }
    }
    for row in legacy.rows(2)? {
        let offset = if half(row, 0)? == 0x64 {
            half(row, 10)? as usize
        } else {
            *row.get(9).ok_or("Truncated artist")? as usize
        };
        db.execute(
            "INSERT INTO artist VALUES (?,?,NULL)",
            params![word(row, 4)?, string(row, offset)?],
        )
        .map_err(error)?;
    }
    for row in legacy.rows(3)? {
        db.execute(
            "INSERT INTO album VALUES (?,?,?,NULL,0,NULL)",
            params![
                word(row, 12)?,
                string(row, *row.get(21).ok_or("Truncated album")? as usize)?,
                word(row, 8)?
            ],
        )
        .map_err(error)?;
    }
    for row in legacy.rows(7)? {
        db.execute(
            "INSERT INTO playlist VALUES (?,?,?,NULL,?,?)",
            params![
                word(row, 12)?,
                word(row, 8)?,
                string(row, 20)?,
                word(row, 16)?,
                word(row, 0)?
            ],
        )
        .map_err(error)?;
    }
    for row in legacy.rows(8)? {
        db.execute(
            "INSERT INTO playlist_content VALUES (?,?,?)",
            params![word(row, 8)?, word(row, 4)?, word(row, 0)?],
        )
        .map_err(error)?;
    }
    for (id, name, _) in COLORS {
        db.execute("INSERT INTO color VALUES (?,?)", params![id, name])
            .map_err(error)?;
    }
    for (id, kind, name) in COLUMNS {
        db.execute(
            "INSERT INTO menuItem VALUES (?,?,?)",
            params![id, kind, name],
        )
        .map_err(error)?;
    }
    for r in CATEGORY_ROWS {
        db.execute(
            "INSERT INTO category VALUES (?,?,?,?)",
            params![r[2], r[0], r[6], r[5] != 1],
        )
        .map_err(error)?;
    }
    for r in SORT_ROWS {
        db.execute(
            "INSERT INTO sort VALUES (?,?,?,?,?)",
            params![r[2], r[0], r[5], r[4] == 0, r[6]],
        )
        .map_err(error)?;
    }
    db.execute(
        "INSERT INTO property VALUES (?,'1000',?,?,0,0)",
        params![
            "Openbox",
            legacy.rows(0)?.len() as i64,
            baken_export::build::today()
        ],
    )
    .map_err(error)?;
    Ok(())
}
