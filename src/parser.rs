use crate::Candidate;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_till1, take_while, take_while1},
    character::complete::{char, line_ending, not_line_ending, space0, space1},
    combinator::{map, opt, recognize, value},
    multi::{many0, many1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
};

// --- ユーティリティパーサー ---

// 改行文字をパースする（Windows/Unix対応）
fn parse_newline(input: &str) -> IResult<&str, &str> {
    alt((tag("\r\n"), tag("\n"))).parse(input)
}

// ゼロ個以上の空白文字やタブをパースする
fn parse_spaces0(input: &str) -> IResult<&str, &str> {
    space0(input)
}

// コメント行（行頭が';'または'#'）をパースする
fn parse_comment(input: &str) -> IResult<&str, &str> {
    recognize((
        parse_spaces0,
        alt((tag(";"), tag("#"))),
        not_line_ending,
        parse_newline,
    ))
    .parse(input)
}

// 空白行をパースする
fn parse_blank_line(input: &str) -> IResult<&str, &str> {
    recognize(tuple((parse_spaces0, parse_newline))).parse(input)
}

// 読み飛ばすべき行（空白行またはコメント行）をパースする
fn parse_ignored_line(input: &str) -> IResult<&str, &str> {
    alt((parse_comment, parse_blank_line)).parse(input)
}

// --- INI要素パーサー ---

/// セクションヘッダをパースし、セクション名（括弧なし）を返す
fn parse_section_header(input: &str) -> IResult<&str, &str> {
    delimited(
        preceded(parse_spaces0, char('[')),
        take_while1(|c| c != ']' && c != '\r' && c != '\n'),
        char(']'),
    )
    .parse(input)
}

/// キーと値のペアをパースし、(キー, 値) のタプルを返す
fn parse_key_value(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(
        // キー: 1文字以上の非空白/非改行文字
        preceded(parse_spaces0, is_not("= \t\r\n")),
        // 区切り: スペース0個、'='、スペース0個
        delimited(parse_spaces0, char('='), parse_spaces0),
        // 値: 改行文字ではないもの全て（末尾の改行は含まない）
        terminated(is_not("\r\n"), parse_newline),
    )
    .parse(input)
}

// --- メインパーサー ---

/// セクションをパースし、セクション名とキーバリューのリストを返す
fn parse_section(input: &str) -> IResult<&str, (&str, Vec<(&str, &str)>)> {
    (
        terminated(parse_section_header, parse_newline), // セクション名
        many0(alt((
            value(("IGNORED", ""), parse_ignored_line), // 読み飛ばす行
            parse_key_value,                            // キーバリューペア
        ))),
    )
        .parse(input)
}

/// INIファイル全体をパースし、セクションのリストを返す
fn parse_ini(input: &str) -> IResult<&str, Vec<(&str, Vec<(&str, &str)>)>> {
    // ファイル先頭の読み飛ばし行を処理
    let (input, _) = many0(parse_ignored_line).parse(input)?;

    // 0個以上のセクションをパース
    many0(parse_section).parse(input)
}

pub fn parse_candicates(ini_content: &str) -> Result<Vec<Candidate>, String> {
    let (_, sections) = parse_ini(ini_content).map_err(|e| format!("Parsing error: {}", e))?;

    let mut result = Vec::<Candidate>::new();

    for (section_name, key_values) in sections {
        if section_name.starts_with("effect.") {
            for (key, value) in key_values {
                if key == "label" {
                    result.push(Candidate {
                        name: key.to_string(),
                        label: value.to_string(),
                    });
                    break;
                }
            }
        }
    }

    Ok(result)
}
