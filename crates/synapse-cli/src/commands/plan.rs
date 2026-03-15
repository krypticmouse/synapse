use std::fs;

pub fn run(file: &str) -> anyhow::Result<()> {
    println!("Planning changes for {file}...\n");

    let source = fs::read_to_string(file)?;
    let program = synapse_dsl::parser::parse(&source)?;

    println!("Memories:");
    for item in &program.items {
        print_item_plan(item, "  ");
    }

    println!("\nNo existing state found. All items will be created.");
    println!("\nRun `synapse apply {file}` to apply these changes.");

    Ok(())
}

fn print_item_plan(item: &synapse_dsl::ast::Item, indent: &str) {
    match item {
        synapse_dsl::ast::Item::Memory(m) => {
            println!("{indent}+ {} (new, {} fields)", m.name, m.fields.len());
        }
        synapse_dsl::ast::Item::Handler(h) => {
            println!("{indent}+ on {} (new handler)", h.event);
        }
        synapse_dsl::ast::Item::Query(q) => {
            println!("{indent}+ query {} (new)", q.name);
        }
        synapse_dsl::ast::Item::Update(u) => {
            println!("{indent}+ update {} ({} rules)", u.target, u.rules.len());
        }
        synapse_dsl::ast::Item::Policy(p) => {
            println!("{indent}+ policy {} ({} rules)", p.name, p.rules.len());
        }
        synapse_dsl::ast::Item::ExternFn(f) => {
            println!("{indent}+ @extern fn {}", f.name);
        }
        synapse_dsl::ast::Item::Namespace(ns) => {
            println!("{indent}namespace {}:", ns.name);
            for item in &ns.items {
                print_item_plan(item, &format!("{indent}  "));
            }
        }
        synapse_dsl::ast::Item::Config(_) => {
            println!("{indent}~ config block");
        }
    }
}
