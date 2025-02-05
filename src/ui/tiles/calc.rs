use gtk4::{self, Builder, ListBoxRow, Label, Box};
use meval::eval_str;

use super::Tile;
use super::util::insert_attrs;

impl Tile{
    pub fn calc_tile(index:i32, equation:&str, method:&String)->(i32, Vec<ListBoxRow>){
        let mut results: Vec<ListBoxRow> = Default::default();
        match eval_str(equation){
            Ok(result)=> {
                let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/calc_tile.ui");

                let holder:ListBoxRow = builder.object("holder").unwrap();
                let attr_holder:Box = builder.object("attrs-holder").unwrap();

                let equation_holder:Label = builder.object("equation-holder").unwrap();
                let result_holder:Label = builder.object("result-holder").unwrap();

                equation_holder.set_text(&equation);
                result_holder.set_text(format!("= {}", result.to_string()).as_str());

                let attrs: Vec<String> = vec![
                    format!("{} | {}", "method", method),
                    format!("{} | {}", "result", result),
                ];
                insert_attrs(&attr_holder, attrs);
                
                results.push(holder);

            }
            _ => {}
        }

        (index, results)
    }
}
