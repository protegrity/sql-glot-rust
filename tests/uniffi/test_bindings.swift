// Exercises the generated Swift bindings against the compiled library.
import Foundation

var failures = 0
var passed = 0

func check(_ condition: Bool, _ name: String, _ detail: @autoclosure () -> String = "") {
    if condition {
        passed += 1
        print("ok   \(name)")
    } else {
        failures += 1
        print("FAIL \(name) \(detail())")
    }
}

func run(_ name: String, _ body: () throws -> Void) {
    do { try body() } catch { check(false, name, "unexpected error: \(error)") }
}

run("version") {
    let expected = ProcessInfo.processInfo.environment["SQLGLOT_EXPECTED_VERSION"] ?? "<unset>"
    check(version() == expected, "version", "\(version()) != \(expected)")
}

run("parse then generate") {
    let out = try parse(sql: "SELECT `select` FROM `events`", dialect: .mysql).generate(dialect: .tsql)
    check(out == "SELECT [select] FROM [events]", "parse then generate", out)
}

run("generate pretty") {
    let out = try parse(sql: "SELECT a, b FROM t WHERE a > 1", dialect: .postgres)
        .generatePretty(dialect: .postgres)
    check(out == "SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  a > 1", "generate pretty", out)
}

run("transpile") {
    let out = try transpile(sql: "SELECT * FROM t LIMIT 10", readDialect: .mysql, writeDialect: .tsql)
    check(out == "SELECT TOP 10 * FROM t", "transpile", out)
}

run("parse statements") {
    let out = try parseStatements(sql: "SELECT 1; SELECT 2", dialect: .ansi).map { try $0.generate(dialect: .ansi) }
    check(out == ["SELECT 1", "SELECT 2"], "parse statements", "\(out)")
}

run("ast access and edit") {
    let statement = try parse(sql: "SELECT a, b FROM t WHERE a > 1", dialect: .postgres)
    var ast = try JSONSerialization.jsonObject(with: Data(try statement.toJson().utf8)) as! [String: Any]
    var select = ast["Select"] as! [String: Any]
    check((select["columns"] as! [Any]).count == 2, "ast column count")

    var from = select["from"] as! [String: Any]
    var source = from["source"] as! [String: Any]
    var table = source["Table"] as! [String: Any]
    check(table["name"] as? String == "t", "ast table name")
    table["name"] = "renamed"
    source["Table"] = table
    from["source"] = source
    select["from"] = from
    ast["Select"] = select

    let json = String(decoding: try JSONSerialization.data(withJSONObject: ast), as: UTF8.self)
    let out = try SqlStatement.fromJson(json: json).generate(dialect: .postgres)
    check(out == "SELECT a, b FROM renamed WHERE a > 1", "ast edit regenerates", out)
}

do {
    _ = try parse(sql: "SELECT FROM WHERE", dialect: .ansi)
    check(false, "parser error", "no error thrown")
} catch SqlglotError.Parser {
    check(true, "parser error")
} catch {
    check(false, "parser error", "wrong error: \(error)")
}

do {
    _ = try parse(sql: "SELECT 'abc", dialect: .ansi)
    check(false, "tokenizer error", "no error thrown")
} catch let SqlglotError.Tokenizer(detail, position) {
    check(detail == "Unterminated string literal" && position == 7, "tokenizer error", "\(detail) @ \(position)")
} catch {
    check(false, "tokenizer error", "wrong error: \(error)")
}

do {
    _ = try parse(sql: "SELECT ARRAY[1, 2]", dialect: .postgres).generate(dialect: .tsql)
    check(false, "unsupported dialect feature", "no error thrown")
} catch let SqlglotError.UnsupportedDialectFeature(detail) {
    check(detail == "ARRAY constructor has no T-SQL equivalent", "unsupported dialect feature", detail)
} catch {
    check(false, "unsupported dialect feature", "wrong error: \(error)")
}

do {
    _ = try SqlStatement.fromJson(json: "{\"Nope\": 1}")
    check(false, "invalid ast", "no error thrown")
} catch SqlglotError.InvalidAst {
    check(true, "invalid ast")
} catch {
    check(false, "invalid ast", "wrong error: \(error)")
}

print("swift: \(passed) passed, \(failures) failed")
exit(failures == 0 && passed > 0 ? 0 : 1)
