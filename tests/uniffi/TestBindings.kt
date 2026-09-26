// Exercises the generated Kotlin bindings against the compiled library.
import uniffi.sqlglot_rust.*
import kotlin.system.exitProcess

var failures = 0
var passed = 0

fun check(condition: Boolean, name: String, detail: String = "") {
    if (condition) {
        passed++
        println("ok   $name")
    } else {
        failures++
        println("FAIL $name $detail")
    }
}

inline fun <reified E : Throwable> expectThrows(name: String, body: () -> Unit): E? {
    try {
        body()
    } catch (e: Throwable) {
        if (e is E) {
            check(true, name)
            return e
        }
        check(false, name, "wrong error: $e")
        return null
    }
    check(false, name, "no error thrown")
    return null
}

fun main() {
    val expected = System.getenv("SQLGLOT_EXPECTED_VERSION") ?: "<unset>"
    check(version() == expected, "version", "${version()} != $expected")
    check(Dialect.entries.size == 30, "all dialects", "${Dialect.entries.size}")

    val quoted = parse("SELECT `select` FROM `events`", Dialect.MYSQL).generate(Dialect.TSQL)
    check(quoted == "SELECT [select] FROM [events]", "parse then generate", quoted)

    val statement = parse("SELECT a, b FROM t WHERE a > 1", Dialect.POSTGRES)
    val pretty = statement.generatePretty(Dialect.POSTGRES)
    check(pretty == "SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  a > 1", "generate pretty", pretty)

    val top = transpile("SELECT * FROM t LIMIT 10", Dialect.MYSQL, Dialect.TSQL)
    check(top == "SELECT TOP 10 * FROM t", "transpile", top)

    val many = parseStatements("SELECT 1; SELECT 2", Dialect.ANSI).map { it.generate(Dialect.ANSI) }
    check(many == listOf("SELECT 1", "SELECT 2"), "parse statements", many.toString())

    val json = statement.toJson()
    val tableName = "\"name\":\"t\""
    check(json.split(tableName).size == 2, "ast has one table named t", json)
    val edited = SqlStatement.fromJson(json.replace(tableName, "\"name\":\"renamed\""))
    val regenerated = edited.generate(Dialect.POSTGRES)
    check(regenerated == "SELECT a, b FROM renamed WHERE a > 1", "ast edit regenerates", regenerated)

    expectThrows<SqlglotException.Parser>("parser error") { parse("SELECT FROM WHERE", Dialect.ANSI) }

    expectThrows<SqlglotException.Tokenizer>("tokenizer error") { parse("SELECT 'abc", Dialect.ANSI) }?.let {
        check(it.detail == "Unterminated string literal" && it.position == 7uL, "tokenizer detail", "${it.detail} @ ${it.position}")
    }

    expectThrows<SqlglotException.UnsupportedDialectFeature>("unsupported dialect feature") {
        parse("SELECT ARRAY[1, 2]", Dialect.POSTGRES).generate(Dialect.TSQL)
    }?.let {
        check(it.detail == "ARRAY constructor has no T-SQL equivalent", "unsupported detail", it.detail)
    }

    expectThrows<SqlglotException.InvalidAst>("invalid ast") { SqlStatement.fromJson("{\"Nope\": 1}") }

    println("kotlin: $passed passed, $failures failed")
    exitProcess(if (failures == 0 && passed > 0) 0 else 1)
}
