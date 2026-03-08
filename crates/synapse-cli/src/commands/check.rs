use std::fs;

pub fn run(file: &str) -> anyhow::Result<()> {
    println!("Checking {file}...");

    let source = fs::read_to_string(file)?;

    // Parse
    let program = match synapse_core::parser::parse(&source) {
        Ok(p) => {
            println!("  ✓ Syntax valid");
            p
        }
        Err(e) => {
            println!("  ✗ Syntax error: {e}");
            return Err(anyhow::anyhow!("syntax check failed"));
        }
    };

    // Type check
    match synapse_core::typeck::check(&program) {
        Ok(_env) => {
            println!("  ✓ Types valid");
        }
        Err(e) => {
            println!("  ✗ Type error: {e}");
            return Err(anyhow::anyhow!("type check failed"));
        }
    }

    // Summarize
    let mut memories = 0;
    let mut handlers = 0;
    let mut queries = 0;
    let mut updates = 0;
    let mut policies = 0;
    let mut extern_fns = 0;

    count_items(
        &program.items,
        &mut memories,
        &mut handlers,
        &mut queries,
        &mut updates,
        &mut policies,
        &mut extern_fns,
    );

    println!(
        "  ✓ {} memories, {} handlers, {} queries, {} update rules, {} policies, {} extern fns",
        memories, handlers, queries, updates, policies, extern_fns
    );

    Ok(())
}

fn count_items(
    items: &[synapse_core::ast::Item],
    memories: &mut usize,
    handlers: &mut usize,
    queries: &mut usize,
    updates: &mut usize,
    policies: &mut usize,
    extern_fns: &mut usize,
) {
    for item in items {
        match item {
            synapse_core::ast::Item::Memory(_) => *memories += 1,
            synapse_core::ast::Item::Handler(_) => *handlers += 1,
            synapse_core::ast::Item::Query(_) => *queries += 1,
            synapse_core::ast::Item::Update(_) => *updates += 1,
            synapse_core::ast::Item::Policy(_) => *policies += 1,
            synapse_core::ast::Item::ExternFn(_) => *extern_fns += 1,
            synapse_core::ast::Item::Namespace(ns) => {
                count_items(
                    &ns.items, memories, handlers, queries, updates, policies, extern_fns,
                );
            }
            synapse_core::ast::Item::Config(_) => {}
        }
    }
}
