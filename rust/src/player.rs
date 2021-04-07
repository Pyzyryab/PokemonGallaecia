pub mod player_mod {
    use gdnative::prelude::*;
    #[derive(Debug)]
    pub struct Player {
        username: String,
        password: String,
        level: u8
    }

    impl ToVariant for Player {
        fn to_variant(&self) -> Variant {
            todo!()
        }
    }
    
    impl Player {  
        // The public constructor
        pub fn create_new_player(username: 
            String, password: String, level: u8) -> Self {

            let player: Player = Player { 
                username: username, 
                password: password, 
                level: level 
            };

            player
        }

        pub fn check_credentials(username: Option<&String>, password: Option<&String>) -> (bool, bool) {

            let mut credentials_flag: (bool, bool) = (false, false);

            // Upgraded flat String credentials to std::option:Option, so pattern matching
            //to make an ez way to scale multiples options when will be checked on a REST-backend
            match username {
                Some(usnm) if usnm == "root" || usnm == "Root" => credentials_flag.0 = true,
                Some(usnm) if usnm == "" => godot_print!("Provide an username"), // While insert an informative label as a child isn't implemented
                Some(_) => (),
                None => panic!(),
            }

            match password {
                Some(pswd) if pswd == "root" || pswd == "Root" => credentials_flag.1 = true,
                Some(pswd) if pswd == "" => godot_print!("Provide a password"),  // While insert an informative label as a child isn't implemented
                Some(_) => (),
                None => panic!() 
            }
            // Returns a tuple representing the checked status of each credential
            credentials_flag
        }

        pub fn credentials_to_rust_string(cred_tup: (GodotString, GodotString)) -> (String, String) {
            let credentials = cred_tup;
            (credentials.0.to_string(), credentials.1.to_string())
        }

    }
}