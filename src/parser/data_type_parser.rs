use crate::ast::{DataType, DataTypeKind};
use crate::dialects::Dialect;
use crate::errors::{Result, SqlglotError};

const TYPE_ALIASES: &[(&str, DataTypeKind)] = &[
    ("AGGREGATEFUNCTION", DataTypeKind::AggregateFunction),
    (
        "SIMPLEAGGREGATEFUNCTION",
        DataTypeKind::SimpleAggregateFunction,
    ),
    ("BIGDECIMAL", DataTypeKind::BigDecimal),
    ("BIGNUMERIC", DataTypeKind::BigDecimal),
    ("BIGINT", DataTypeKind::BigInt),
    ("BIGNUM", DataTypeKind::BigNum),
    ("BIT", DataTypeKind::Bit),
    ("BOOL", DataTypeKind::Boolean),
    ("BOOLEAN", DataTypeKind::Boolean),
    ("BYTE", DataTypeKind::TinyInt),
    ("BYTEA", DataTypeKind::Varbinary),
    ("CHAR VARYING", DataTypeKind::Varchar),
    ("CHARACTER", DataTypeKind::Char),
    ("CHARACTER VARYING", DataTypeKind::Varchar),
    ("CLOB", DataTypeKind::Text),
    ("DATE", DataTypeKind::Date),
    ("DATEMULTIRANGE", DataTypeKind::DateMultiRange),
    ("DATERANGE", DataTypeKind::DateRange),
    ("DEC", DataTypeKind::Decimal),
    ("DECFLOAT", DataTypeKind::DecFloat),
    ("DECIMAL", DataTypeKind::Decimal),
    ("DECIMAL128", DataTypeKind::Decimal128),
    ("DECIMAL256", DataTypeKind::Decimal256),
    ("DECIMAL32", DataTypeKind::Decimal32),
    ("DECIMAL64", DataTypeKind::Decimal64),
    ("DOUBLE", DataTypeKind::Double),
    ("DOUBLE PRECISION", DataTypeKind::Double),
    ("ENUM", DataTypeKind::Enum),
    ("FIXED", DataTypeKind::Decimal),
    ("FLOAT4", DataTypeKind::Float),
    ("FLOAT8", DataTypeKind::Double),
    ("GEOGRAPHY", DataTypeKind::Geography),
    ("GEOMETRY", DataTypeKind::Geometry),
    ("HUGEINT", DataTypeKind::Int128),
    ("INET", DataTypeKind::Inet),
    ("INT", DataTypeKind::Int),
    ("INT1", DataTypeKind::TinyInt),
    ("INT128", DataTypeKind::Int128),
    ("INT16", DataTypeKind::SmallInt),
    ("INT2", DataTypeKind::SmallInt),
    ("INT256", DataTypeKind::Int256),
    ("INT32", DataTypeKind::Int),
    ("INT4", DataTypeKind::Int),
    ("INT4MULTIRANGE", DataTypeKind::Int4MultiRange),
    ("INT4RANGE", DataTypeKind::Int4Range),
    ("INT64", DataTypeKind::BigInt),
    ("INT8MULTIRANGE", DataTypeKind::Int8MultiRange),
    ("INT8RANGE", DataTypeKind::Int8Range),
    ("INTEGER", DataTypeKind::Int),
    ("INTERVAL", DataTypeKind::Interval),
    ("JSON", DataTypeKind::Json),
    ("JSONB", DataTypeKind::Jsonb),
    ("LIST", DataTypeKind::List),
    ("LONG", DataTypeKind::BigInt),
    ("LONGBLOB", DataTypeKind::LongBlob),
    ("LONGTEXT", DataTypeKind::LongText),
    ("LONGVARCHAR", DataTypeKind::Text),
    ("MAP", DataTypeKind::Map),
    ("MEDIUMBLOB", DataTypeKind::MediumBlob),
    ("MEDIUMINT", DataTypeKind::MediumInt),
    ("MEDIUMTEXT", DataTypeKind::MediumText),
    ("NCHAR", DataTypeKind::NChar),
    ("NULL", DataTypeKind::Null),
    ("NUMBER", DataTypeKind::Decimal),
    ("NUMERIC", DataTypeKind::Decimal),
    ("NUMMULTIRANGE", DataTypeKind::NumMultiRange),
    ("NUMRANGE", DataTypeKind::NumRange),
    ("NVARCHAR", DataTypeKind::Nvarchar),
    ("NVARCHAR2", DataTypeKind::Nvarchar),
    ("OBJECT", DataTypeKind::Object),
    ("RANGE", DataTypeKind::Range),
    ("REAL", DataTypeKind::Float),
    ("SHORT", DataTypeKind::SmallInt),
    ("SMALLINT", DataTypeKind::SmallInt),
    ("STR", DataTypeKind::Text),
    ("STRING", DataTypeKind::Text),
    ("STRUCT", DataTypeKind::Struct),
    ("TEXT", DataTypeKind::Text),
    ("TIME", DataTypeKind::Time),
    ("TIMESTAMPLTZ", DataTypeKind::TimestampLtz),
    ("TIMESTAMPNTZ", DataTypeKind::TimestampNtz),
    ("TIMESTAMPTZ", DataTypeKind::TimestampTz),
    ("TIMESTAMP_LTZ", DataTypeKind::TimestampLtz),
    ("TIMESTAMP_NTZ", DataTypeKind::TimestampNtz),
    ("TIMETZ", DataTypeKind::TimeTz),
    ("TIME_NS", DataTypeKind::TimeNs),
    ("TINYBLOB", DataTypeKind::TinyBlob),
    ("TINYTEXT", DataTypeKind::TinyText),
    ("TSMULTIRANGE", DataTypeKind::TsMultiRange),
    ("TSRANGE", DataTypeKind::TsRange),
    ("TSTZMULTIRANGE", DataTypeKind::TstzMultiRange),
    ("TSTZRANGE", DataTypeKind::TstzRange),
    ("UHUGEINT", DataTypeKind::UInt128),
    ("UINT", DataTypeKind::UInt),
    ("UINT128", DataTypeKind::UInt128),
    ("UINT256", DataTypeKind::UInt256),
    ("UNION", DataTypeKind::Union),
    ("UNKNOWN", DataTypeKind::Unknown),
    ("USER-DEFINED", DataTypeKind::UserDefined),
    ("UUID", DataTypeKind::Uuid),
    ("VARBINARY", DataTypeKind::Varbinary),
    ("VARCHAR2", DataTypeKind::Varchar),
    ("VARIANT", DataTypeKind::Variant),
    ("VECTOR", DataTypeKind::Vector),
];

pub(crate) fn parse(sql: &str, dialect: Dialect) -> Result<DataType> {
    let sql = sql.trim();
    if sql.is_empty() {
        return parser_error("Expected data type");
    }

    if let Some(inner) = nullable_inner(sql) {
        let data_type = parse(inner, dialect)?;
        return Ok(DataType::Dialect {
            kind: data_type.kind(),
            sql: sql.to_string(),
        });
    }

    let normalized = normalize_words(sql);
    let (mut kind, alias_len) =
        longest_alias(&normalized, dialect).ok_or_else(|| SqlglotError::ParserError {
            message: format!("Unknown data type '{sql}'"),
        })?;

    let tail = normalized[alias_len..].trim_start();
    validate_tail(tail)?;
    if let Some(wrapper) = postfix_kind(tail) {
        kind = wrapper;
    }
    kind = apply_modifiers(kind, tail);

    Ok(build_data_type(kind, sql))
}

pub(crate) fn parse_schema(sql: &str, dialect: Dialect) -> Result<DataType> {
    parse_with_udt(sql, dialect, dialect.supports_user_defined_types())
}

pub(crate) fn parse_with_udt(sql: &str, dialect: Dialect, udt: bool) -> Result<DataType> {
    match parse(sql, dialect) {
        Ok(data_type) => Ok(data_type),
        Err(_) if udt && valid_user_defined_type(sql) => {
            Ok(DataType::UserDefined(sql.trim().to_string()))
        }
        Err(error) => Err(error),
    }
}

fn longest_alias(sql: &str, dialect: Dialect) -> Option<(DataTypeKind, usize)> {
    let mut best = None;
    for end in word_ends(sql) {
        let spelling = &sql[..end];
        if let Some(kind) = dialect_kind(spelling, dialect).or_else(|| base_kind(spelling)) {
            best = Some((kind, end));
        }
    }
    best
}

fn base_kind(spelling: &str) -> Option<DataTypeKind> {
    let default = match spelling {
        "ARRAY" => DataTypeKind::Array,
        "BINARY" => DataTypeKind::Binary,
        "BLOB" => DataTypeKind::Varbinary,
        "BPCHAR" => DataTypeKind::BpChar,
        "CHAR" => DataTypeKind::Char,
        "DATETIME" => DataTypeKind::DateTime,
        "FLOAT" => DataTypeKind::Float,
        "INT8" => DataTypeKind::TinyInt,
        "TIMESTAMP" => DataTypeKind::Timestamp,
        "TINYINT" => DataTypeKind::TinyInt,
        "VARCHAR" => DataTypeKind::Varchar,
        _ => {
            return TYPE_ALIASES
                .iter()
                .find_map(|(alias, kind)| (*alias == spelling).then_some(*kind));
        }
    };
    Some(default)
}

#[allow(clippy::too_many_lines)]
fn dialect_kind(spelling: &str, dialect: Dialect) -> Option<DataTypeKind> {
    use DataTypeKind as K;
    use Dialect as D;

    match (spelling, dialect) {
        ("BINARY", D::DuckDb) => Some(K::Varbinary),
        ("BLOB", D::Doris | D::Mysql | D::SingleStore | D::StarRocks) => Some(K::Blob),
        ("BPCHAR" | "CHAR" | "VARCHAR", D::DuckDb) => Some(K::Text),
        ("DATETIME", D::BigQuery) => Some(K::Timestamp),
        ("DATETIME", D::DuckDb) => Some(K::TimestampNtz),
        ("FLOAT", D::Materialize | D::Postgres | D::Redshift | D::RisingWave | D::Snowflake) => {
            Some(K::Double)
        }
        ("INT8", D::DuckDb | D::Materialize | D::Postgres | D::Redshift | D::RisingWave) => {
            Some(K::BigInt)
        }
        (
            "TIMESTAMP",
            D::BigQuery | D::Databricks | D::Doris | D::Mysql | D::Spark | D::StarRocks,
        ) => Some(K::TimestampTz),
        ("TIMESTAMP", D::DuckDb) => Some(K::TimestampNtz),
        ("TIMESTAMP", D::Tsql) => Some(K::RowVersion),
        ("TINYINT", D::Fabric | D::Tsql) => Some(K::UTinyInt),

        ("ANY TYPE", D::BigQuery) => Some(K::Variant),
        ("BYTEINT", D::BigQuery | D::Snowflake) => Some(K::Int),
        ("BYTEINT", D::Teradata) => Some(K::SmallInt),
        ("BYTES", D::BigQuery) => Some(K::Binary),
        ("FLOAT64", D::BigQuery | D::ClickHouse) => Some(K::Double),
        ("RECORD", D::BigQuery | D::SingleStore) => Some(K::Struct),

        ("DATE32", D::ClickHouse) => Some(K::Date32),
        ("DATETIME64", D::ClickHouse) => Some(K::DateTime64),
        ("DYNAMIC", D::ClickHouse) => Some(K::Dynamic),
        ("ENUM8", D::ClickHouse) => Some(K::Enum8),
        ("ENUM16", D::ClickHouse) => Some(K::Enum16),
        ("FIXEDSTRING", D::ClickHouse) => Some(K::FixedString),
        ("FLOAT32", D::ClickHouse) => Some(K::Float),
        ("IPV4", D::ClickHouse) => Some(K::Ipv4),
        ("IPV6", D::ClickHouse) => Some(K::Ipv6),
        ("LINESTRING", D::ClickHouse) => Some(K::LineString),
        ("LOWCARDINALITY", D::ClickHouse) => Some(K::LowCardinality),
        ("MULTILINESTRING", D::ClickHouse) => Some(K::MultiLineString),
        ("MULTIPOLYGON", D::ClickHouse) => Some(K::MultiPolygon),
        ("NESTED", D::ClickHouse) => Some(K::Nested),
        ("NOTHING", D::ClickHouse) => Some(K::Nothing),
        ("POINT", D::ClickHouse | D::Materialize | D::Postgres | D::Redshift | D::RisingWave) => {
            Some(K::Point)
        }
        ("POLYGON", D::ClickHouse) => Some(K::Polygon),
        ("RING", D::ClickHouse) => Some(K::Ring),
        ("TUPLE", D::ClickHouse) => Some(K::Struct),
        ("UINT8", D::ClickHouse) => Some(K::UTinyInt),
        ("UINT16", D::ClickHouse) => Some(K::USmallInt),
        ("UINT32", D::ClickHouse) => Some(K::UInt),
        ("UINT64", D::ClickHouse) => Some(K::UBigInt),

        ("VOID", D::Databricks) => Some(K::Null),
        (
            "SERIAL",
            D::Doris
            | D::Materialize
            | D::Mysql
            | D::Postgres
            | D::Redshift
            | D::RisingWave
            | D::SingleStore
            | D::StarRocks,
        ) => Some(K::Serial),
        ("SET", D::Doris | D::Mysql | D::SingleStore | D::StarRocks) => Some(K::Set),
        ("SIGNED" | "SIGNED INTEGER", D::Doris | D::Mysql | D::SingleStore | D::StarRocks) => {
            Some(K::BigInt)
        }
        ("UNSIGNED" | "UNSIGNED INTEGER", D::Doris | D::Mysql | D::SingleStore | D::StarRocks) => {
            Some(K::UBigInt)
        }
        ("YEAR", D::Doris | D::Mysql | D::SingleStore | D::StarRocks) => Some(K::Year),

        ("BITSTRING", D::DuckDb) => Some(K::Bit),
        ("LOGICAL", D::DuckDb) => Some(K::Boolean),
        ("ROW", D::DuckDb | D::Athena | D::Presto | D::Trino) => Some(K::Struct),
        ("SIGNED", D::DuckDb) => Some(K::Int),
        ("TIMESTAMP_MS", D::DuckDb) => Some(K::TimestampMs),
        ("TIMESTAMP_NS", D::DuckDb) => Some(K::TimestampNs),
        ("TIMESTAMP_S", D::DuckDb) => Some(K::TimestampS),
        ("TIMESTAMP_US", D::DuckDb) => Some(K::Timestamp),
        ("UBIGINT", D::DuckDb) => Some(K::UBigInt),
        ("UINTEGER", D::DuckDb) => Some(K::UInt),
        ("USMALLINT", D::DuckDb) => Some(K::USmallInt),
        ("UTINYINT", D::DuckDb) => Some(K::UTinyInt),
        ("LONG VARCHAR", D::Exasol) => Some(K::Text),

        ("DATETIME2", D::Fabric | D::Tsql) => Some(K::DateTime2),
        ("DATETIMEOFFSET", D::Fabric | D::Tsql) => Some(K::TimestampTz),
        ("IMAGE", D::Fabric | D::Tsql) => Some(K::Image),
        (
            "MONEY",
            D::Fabric | D::Materialize | D::Postgres | D::Redshift | D::RisingWave | D::Tsql,
        ) => Some(K::Money),
        ("NTEXT", D::Fabric | D::Tsql) => Some(K::Text),
        ("ROWVERSION", D::Fabric | D::Tsql) => Some(K::RowVersion),
        ("SMALLDATETIME", D::Fabric | D::Tsql) => Some(K::SmallDateTime),
        ("SMALLMONEY", D::Fabric | D::Tsql) => Some(K::SmallMoney),
        ("SQL_VARIANT", D::Fabric | D::Tsql) => Some(K::Variant),
        ("UNIQUEIDENTIFIER", D::Fabric | D::Tsql) => Some(K::Uuid),
        ("UTINYINT", D::Fabric) => Some(K::UTinyInt),
        (
            "XML",
            D::Fabric | D::Materialize | D::Postgres | D::Redshift | D::RisingWave | D::Tsql,
        ) => Some(K::Xml),

        ("BIGSERIAL", D::Materialize | D::Postgres | D::Redshift | D::RisingWave) => {
            Some(K::BigSerial)
        }
        ("HSTORE", D::Materialize | D::Postgres | D::Redshift | D::RisingWave) => Some(K::HStore),
        ("NAME", D::Materialize | D::Postgres | D::Redshift | D::RisingWave) => Some(K::Name),
        ("SMALLSERIAL", D::Materialize | D::Postgres | D::Redshift | D::RisingWave) => {
            Some(K::SmallSerial)
        }
        (
            "CSTRING" | "OID" | "REGCLASS" | "REGCOLLATION" | "REGCONFIG" | "REGDICTIONARY"
            | "REGNAMESPACE" | "REGOPER" | "REGOPERATOR" | "REGPROC" | "REGPROCEDURE" | "REGROLE"
            | "REGTYPE",
            D::Materialize | D::Postgres | D::Redshift | D::RisingWave,
        ) => Some(K::UserDefined),

        ("BINARY_DOUBLE" | "SQL_DOUBLE", D::Oracle | D::Snowflake) => Some(K::Double),
        ("BINARY_FLOAT", D::Oracle) => Some(K::Float),
        ("BINARY VARYING", D::Redshift) => Some(K::Varbinary),
        ("HLLSKETCH", D::Redshift) => Some(K::HllSketch),
        ("SUPER", D::Redshift) => Some(K::Super),
        ("VARBYTE", D::Redshift) => Some(K::Varbinary),
        ("BSON", D::SingleStore) => Some(K::Jsonb),
        ("GEOGRAPHYPOINT", D::SingleStore) => Some(K::GeographyPoint),
        ("LARGEINT", D::StarRocks) => Some(K::Int128),
        ("FILE", D::Snowflake) => Some(K::File),
        ("NCHAR VARYING" | "SQL_VARCHAR", D::Snowflake) => Some(K::Varchar),
        ("TIMESTAMP_TZ", D::Snowflake) => Some(K::TimestampTz),
        ("ST_GEOMETRY", D::Teradata) => Some(K::Geometry),

        ("HYPERLOGLOG", D::Athena | D::Presto | D::Trino) => Some(K::HllSketch),
        ("IPADDRESS", D::Athena | D::Presto | D::Trino) => Some(K::IpAddress),
        ("IPPREFIX", D::Athena | D::Presto | D::Trino) => Some(K::IpPrefix),
        ("TDIGEST", D::Athena | D::Presto | D::Trino) => Some(K::TDigest),
        _ => None,
    }
}

fn build_data_type(kind: DataTypeKind, sql: &str) -> DataType {
    let (precision, scale) = numeric_parameters(sql);
    match kind {
        DataTypeKind::TinyInt => DataType::TinyInt,
        DataTypeKind::SmallInt => DataType::SmallInt,
        DataTypeKind::Int => DataType::Int,
        DataTypeKind::BigInt => DataType::BigInt,
        DataTypeKind::Float => DataType::Float,
        DataTypeKind::Double => DataType::Double,
        DataTypeKind::Decimal => DataType::Decimal { precision, scale },
        DataTypeKind::Varchar => DataType::Varchar(precision),
        DataTypeKind::Char => DataType::Char(precision),
        DataTypeKind::Text => DataType::Text,
        DataTypeKind::Binary => DataType::Binary(precision),
        DataTypeKind::Varbinary => DataType::Varbinary(precision),
        DataTypeKind::Blob => DataType::Blob,
        DataTypeKind::NChar => DataType::NChar(precision),
        DataTypeKind::Nvarchar => DataType::NVarchar(precision),
        DataTypeKind::Boolean => DataType::Boolean,
        DataTypeKind::Date => DataType::Date,
        DataTypeKind::Time => DataType::Time { precision },
        DataTypeKind::Timestamp => DataType::Timestamp {
            precision,
            with_tz: false,
        },
        DataTypeKind::TimestampTz => DataType::Timestamp {
            precision,
            with_tz: true,
        },
        DataTypeKind::Interval => DataType::Interval,
        DataTypeKind::DateTime => DataType::DateTime,
        DataTypeKind::Json => DataType::Json,
        DataTypeKind::Jsonb => DataType::Jsonb,
        DataTypeKind::Uuid => DataType::Uuid,
        DataTypeKind::Null => DataType::Null,
        DataTypeKind::Variant => DataType::Variant,
        DataTypeKind::Object => DataType::Object,
        DataTypeKind::Xml => DataType::Xml,
        DataTypeKind::Inet => DataType::Inet,
        DataTypeKind::Bit => DataType::Bit(precision),
        DataTypeKind::Money => DataType::Money,
        DataTypeKind::Serial => DataType::Serial,
        DataTypeKind::BigSerial => DataType::BigSerial,
        DataTypeKind::SmallSerial => DataType::SmallSerial,
        DataTypeKind::HStore => DataType::Hstore,
        DataTypeKind::Geography => DataType::Geography,
        DataTypeKind::Geometry => DataType::Geometry,
        DataTypeKind::Super => DataType::Super,
        DataTypeKind::Unknown => DataType::Unknown("UNKNOWN".to_string()),
        DataTypeKind::UserDefined => DataType::UserDefined(sql.to_string()),
        _ => DataType::Dialect {
            kind,
            sql: sql.to_string(),
        },
    }
}

fn apply_modifiers(kind: DataTypeKind, tail: &str) -> DataTypeKind {
    let tail = tail.to_ascii_uppercase();
    if tail.contains("UNSIGNED") {
        return match kind {
            DataTypeKind::BigInt => DataTypeKind::UBigInt,
            DataTypeKind::Int => DataTypeKind::UInt,
            DataTypeKind::MediumInt => DataTypeKind::UMediumInt,
            DataTypeKind::SmallInt => DataTypeKind::USmallInt,
            DataTypeKind::TinyInt => DataTypeKind::UTinyInt,
            DataTypeKind::Decimal => DataTypeKind::UDecimal,
            DataTypeKind::Double => DataTypeKind::UDouble,
            kind => kind,
        };
    }
    if tail.contains("WITH LOCAL TIME ZONE") {
        DataTypeKind::TimestampLtz
    } else if tail.contains("WITH TIME ZONE") {
        match kind {
            DataTypeKind::Time => DataTypeKind::TimeTz,
            DataTypeKind::Timestamp => DataTypeKind::TimestampTz,
            kind => kind,
        }
    } else {
        kind
    }
}

fn postfix_kind(tail: &str) -> Option<DataTypeKind> {
    let mut depth = 0;
    let mut word = String::new();
    for c in tail.chars() {
        match c {
            '(' | '<' => depth += 1,
            ')' | '>' => depth -= 1,
            '[' if depth == 0 => return Some(DataTypeKind::Array),
            '[' => depth += 1,
            ']' => depth -= 1,
            c if depth == 0 && (c.is_ascii_alphabetic() || c == '_') => {
                word.push(c.to_ascii_uppercase());
            }
            c if depth == 0 && !word.is_empty() => {
                if word == "ARRAY" {
                    return Some(DataTypeKind::Array);
                }
                if word == "LIST" {
                    return Some(DataTypeKind::List);
                }
                word.clear();
                if !c.is_whitespace() {
                    continue;
                }
            }
            _ => {}
        }
    }
    match word.as_str() {
        "ARRAY" => Some(DataTypeKind::Array),
        "LIST" => Some(DataTypeKind::List),
        _ => None,
    }
}

fn validate_tail(tail: &str) -> Result<()> {
    let mut stack = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut top_level_words = String::new();

    for c in tail.chars() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == delimiter {
                quote = None;
            }
            continue;
        }

        match c {
            '\'' | '"' | '`' => quote = Some(c),
            '(' | '<' | '[' => stack.push(c),
            ')' => {
                if stack.pop() != Some('(') {
                    return parser_error("Unbalanced data type");
                }
            }
            '>' => {
                if stack.pop() != Some('<') {
                    return parser_error("Unbalanced data type");
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return parser_error("Unbalanced data type");
                }
            }
            ';' => return parser_error("Unexpected statement terminator in data type"),
            c if stack.is_empty() && (c.is_ascii_alphabetic() || c == '_') => {
                top_level_words.push(c.to_ascii_uppercase());
            }
            c if stack.is_empty() && c.is_whitespace() => top_level_words.push(' '),
            c if c.is_ascii_alphanumeric() || c.is_whitespace() || "_.,=:+-*/".contains(c) => {}
            _ => return parser_error(&format!("Unexpected token '{c}' in data type")),
        }
    }

    if quote.is_some() || !stack.is_empty() {
        return parser_error("Unbalanced data type");
    }

    let words = normalize_words(&top_level_words);
    if words.is_empty() || valid_suffix(&words) {
        Ok(())
    } else {
        parser_error(&format!("Unexpected data type suffix '{words}'"))
    }
}

fn valid_suffix(words: &str) -> bool {
    const SUFFIX_WORDS: &[&str] = &[
        "ARRAY",
        "BYTE",
        "CHAR",
        "CHARACTER",
        "COLLATE",
        "DAY",
        "ENCODING",
        "FORMAT",
        "HOUR",
        "LIST",
        "LOCAL",
        "MINUTE",
        "MONTH",
        "PRECISION",
        "SECOND",
        "SET",
        "TIME",
        "TO",
        "UNSIGNED",
        "VARYING",
        "WITH",
        "WITHOUT",
        "YEAR",
        "ZONE",
    ];
    words
        .split_whitespace()
        .all(|word| SUFFIX_WORDS.contains(&word))
}

fn numeric_parameters(sql: &str) -> (Option<u32>, Option<u32>) {
    let Some(start) = sql.find('(') else {
        return (None, None);
    };
    let Some(end) = sql[start + 1..].find(')') else {
        return (None, None);
    };
    let values: Vec<_> = sql[start + 1..start + 1 + end]
        .split(',')
        .map(str::trim)
        .collect();
    (
        values.first().and_then(|value| value.parse().ok()),
        values.get(1).and_then(|value| value.parse().ok()),
    )
}

fn normalize_words(sql: &str) -> String {
    sql.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

fn word_ends(sql: &str) -> impl Iterator<Item = usize> + '_ {
    sql.char_indices()
        .filter_map(|(index, c)| {
            ((c.is_whitespace() || "([<".contains(c)) && index > 0).then_some(index)
        })
        .chain(std::iter::once(sql.len()))
}

fn nullable_inner(sql: &str) -> Option<&str> {
    let upper = sql.to_ascii_uppercase();
    if !upper.starts_with("NULLABLE(") || !sql.ends_with(')') {
        return None;
    }
    Some(&sql["NULLABLE(".len()..sql.len() - 1])
}

fn valid_user_defined_type(sql: &str) -> bool {
    let mut sql = sql.trim();
    while let Some(base) = strip_array_suffix(sql) {
        sql = base.trim_end();
    }
    if sql.is_empty() {
        return false;
    }

    sql.split('.').all(valid_identifier)
}

fn strip_array_suffix(sql: &str) -> Option<&str> {
    if !sql.ends_with(']') {
        return None;
    }
    let start = sql.rfind('[')?;
    if start == 0
        || !sql[start + 1..sql.len() - 1]
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace())
    {
        return None;
    }
    Some(&sql[..start])
}

fn valid_identifier(identifier: &str) -> bool {
    let identifier = identifier.trim();
    if identifier.len() >= 2 {
        let first = identifier.as_bytes()[0] as char;
        let last = identifier.as_bytes()[identifier.len() - 1] as char;
        if matches!((first, last), ('"', '"') | ('`', '`') | ('[', ']')) {
            return !identifier[1..identifier.len() - 1].is_empty();
        }
    }

    let mut chars = identifier.chars();
    matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
}

fn parser_error<T>(message: &str) -> Result<T> {
    Err(SqlglotError::ParserError {
        message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_schema};
    use crate::ast::{DataType, DataTypeKind};
    use crate::dialects::Dialect;

    #[test]
    fn applies_dialect_type_mappings() {
        assert_eq!(parse("INT8", Dialect::Postgres).unwrap(), DataType::BigInt);
        assert_eq!(parse("INT8", Dialect::Ansi).unwrap(), DataType::TinyInt);
        assert_eq!(
            parse("TIMESTAMP", Dialect::BigQuery).unwrap().kind(),
            DataTypeKind::TimestampTz
        );
        assert_eq!(
            parse("TIMESTAMP", Dialect::Tsql).unwrap().kind(),
            DataTypeKind::RowVersion
        );
    }

    #[test]
    fn parses_nested_and_parameterized_types() {
        assert_eq!(
            parse("MAP<VARCHAR, ARRAY<DECIMAL(38, 9)>>", Dialect::Trino)
                .unwrap()
                .kind(),
            DataTypeKind::Map
        );
        assert_eq!(
            parse("Nullable(LowCardinality(String))", Dialect::ClickHouse)
                .unwrap()
                .kind(),
            DataTypeKind::LowCardinality
        );
        assert_eq!(
            parse("INTERVAL DAY(3) TO SECOND(6)", Dialect::Oracle)
                .unwrap()
                .kind(),
            DataTypeKind::Interval
        );
    }

    #[test]
    fn distinguishes_user_defined_types_from_invalid_builtins() {
        assert!(matches!(
            parse_schema("app.custom_type", Dialect::Postgres).unwrap(),
            DataType::UserDefined(_)
        ));
        assert!(parse_schema("custom_type", Dialect::BigQuery).is_err());
        assert!(parse("INTEGER nonsense", Dialect::Postgres).is_err());
        assert!(parse("ARRAY<INTEGER", Dialect::Postgres).is_err());
    }
}
