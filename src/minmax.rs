pub mod mod_min_max {
    use std::fmt;

    #[derive(Default, Debug, Clone, PartialEq)]
    pub struct MinMax {
        pub nb: i32,
        pub min: i32,
        pub max: i32,
        pub last: i32,
        pub ordre: bool,
    }

    impl MinMax {
        pub(crate) fn add(&mut self, valeur: i32) {
            if self.nb == 0 {
                self.min = valeur;
                self.max = valeur;
            } else {
                if self.min > valeur {
                    self.min = valeur;
                }
                if self.max < valeur {
                    self.max = valeur;
                }
            }
            self.nb += 1;
            if self.ordre {
                if self.last > valeur {
                    self.ordre = false;
                }
            }
            self.last = valeur;
        }
    }


    impl fmt::Display for MinMax {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "({},{},{})", self.min, self.max, self.ordre)
        }
    }

    pub fn create_min_max() -> MinMax {
        MinMax {
            nb: 0,
            min: 0,
            max: 0,
            last: 0,
            ordre: true,
        }
    }

}