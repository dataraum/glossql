//! The glossql server binary: DataFusion session + statement routing +
//! Flight SQL transport (actor in the handshake) + the MCP shim exposing
//! one `glossql(statements)` tool. Milestone 1 starts with statement
//! routing over stdin; transport joins in milestone 4.

fn main() {
    // Scaffold smoke: route one statement through the parser.
    let demo = "USE fin;";
    match parser::tokenize(demo).map(parser::split_statements) {
        Ok(stmts) => println!(
            "glossql serverd scaffold — parsed {} statement(s) from {demo:?}",
            stmts.len()
        ),
        Err(e) => eprintln!("parse error: {e}"),
    }
}
