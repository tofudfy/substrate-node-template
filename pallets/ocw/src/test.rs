use sp_arithmetic::per_things::Permill;
use scale_info::prelude::string::String;

#[test]
fn Permill_test() {
	let integer: u64 = 22;
	let decimal: u32 = 604569; // 604569863

	format_price((integer, Permill::from_perthousand(decimal)));

	format_price((integer, Permill::from_parts(decimal)));
}

type Price = (u64, Permill);


fn format_price(data: Price) -> String {
	let integer = data.0;
	let decimal = data.1.deconstruct();
	println!("Price parsing result: ({}, {})", integer, decimal);

	return integer.to_string() + "." + &decimal.to_string();
}
