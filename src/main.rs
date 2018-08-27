
use humansize::{file_size_opts, FileSize};
use std::path::PathBuf;



pub(crate) struct DirSizes {
    numb_reg_src_checkouts: u64, // number of source checkouts
    total_reg_src_size: u64,
}

impl DirSizes {
    pub(crate) fn print_pretty(&self, cache_root_dir: &PathBuf) -> String {
        fn pad_strings(indent_lvl: i8, beginning: &str, end: &str) -> String {
            const MAX_WIDTH: i8 = 37;

            let len_padding: i8 = (MAX_WIDTH + indent_lvl * 2) - (beginning.len() as i8);
            let mut formatted_line = beginning.to_string();
            formatted_line.push_str(&String::from(" ").repeat(len_padding as usize));
            formatted_line.push_str(&end);
            formatted_line.push_str("\n");
            formatted_line
        }

        let mut s = String::new();

        s.push_str(&pad_strings(
            2,
            &format!(
                "Size of {} crate source checkouts: ",
                self.numb_reg_src_checkouts
            ),
            &self
                .total_reg_src_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s
    }
}


fn main() {

    impl DirSizes {
        #[allow(non_snake_case)]
        pub(crate) fn new_manually(a: u64, b: u64) -> Self {

            Self {
                total_reg_src_size: a,
                numb_reg_src_checkouts: b,
            }
        }
    }



        // create a DirSizes object
        let dirSizes = DirSizes::new_manually(1_938_493_989, 123_909_849);

        let cache_root = PathBuf::from("/home/user/.cargo");
        let output_is = dirSizes.print_pretty(&cache_root);

        let output_should = "Cargo cache '/home/user/.cargo/':
Size of 123909849 crate source checkouts:1.94 GB";

        assert_eq!(output_is, output_should);


}
