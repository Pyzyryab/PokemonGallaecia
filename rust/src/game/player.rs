use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde::ser::Serializer;

use gdnative::prelude::*;
use gdnative::api::{AnimatedSprite, KinematicBody2D, KinematicCollision2D};

use crate::{game::dialogue_box::DialogueBoxStatus};
use crate::game::code_abstractions::{
    character::CharacterMovement,
    signals::GodotSignal,
    signals::RegisterSignal
};

use crate::utils::utils;
use crate::utils::consts::in_game_constant;

use super::menu::menu::MenuStatus;


#[derive(Serialize, Deserialize, Debug)]
/// This beautiful struct is the responsable of read the data coming from signals of all 
/// different PLAYER "classes", processing that data and store it on an external resource
/// where the data can persist
pub struct PlayerData {
    name: String, // All JSON attrs has a 'name' identifier depending on what kind of data are storing
    player_direction: PlayerDirection,
    player_position: HashMap<String, f64>,
}

impl PlayerData {

    pub fn new() -> Self {
        Self {
            name: "".to_owned(),
            player_direction: PlayerDirection::default(),
            player_position: HashMap::new(),
        }
    }

    pub fn set_player_direction(&mut self, player_current_direction: &PlayerDirection) {
        self.player_direction = player_current_direction.to_owned();
    }
    pub fn set_player_position(&mut self, x: f64, y: f64) {
        self.player_position.insert("x".to_owned(), x);
        self.player_position.insert("y".to_owned(), y);
    }
}


#[derive(NativeClass)]
#[inherit(KinematicBody2D)]
#[register_with(Self::register_signal)]
#[derive(Debug)]
pub struct PlayerCharacter {
    player_status: PlayerStatus,
    menu_status: MenuStatus,
    dialogue_box_status: DialogueBoxStatus,
    motion: Vector2, // A Vector2, which is a Godot type, in this case, represents and tracks the (x, y) coordinates on 2D space
    signals: HashMap<String, GodotSignal<'static>>,
    current_position: Vector2,
    counter: i32,
}

impl RegisterSignal<Self> for PlayerCharacter {
    fn register_signal(builder: &ClassBuilder<Self>) {
        // Indicates that the Player is moving
        builder.add_signal( Signal {
            name: "animate",
            args: &[ SignalArgument {
                name: "motion",
                default: Variant::from_vector2(&Vector2::new(0.0, 0.0)),
                export_info: ExportInfo::new(VariantType::Vector2),
                usage: PropertyUsage::DEFAULT,
            }],
        });

        // Indicates that the Player is interacting
        builder.add_signal( Signal {
            name: "player_interacting",
            args: &[],
        });

        builder.add_signal( Signal {
            name: "player_position",
            args: &[]
        });
    }
}

impl CharacterMovement<KinematicBody2D, Input>  for PlayerCharacter {
    /// The fn that manages the player motion on the `Map`, and updates the `self.player_status: PlayerStatus`, 
    /// which represents the current variant of the player different status and behaviours. 
    fn move_character(&mut self, _owner: &KinematicBody2D, input: &Input) 
    {
        if Input::is_action_pressed(&input, "Left") {
            self.motion.x = in_game_constant::VELOCITY * -1.0;
            self.motion.y = 0.0;
            self.player_status = PlayerStatus::Walking    
        } 
        else if Input::is_action_pressed(&input, "Right") {
            self.motion.x = in_game_constant::VELOCITY;
            self.motion.y = 0.0;
            self.player_status = PlayerStatus::Walking 
        } 
        else if Input::is_action_pressed(&input, "Up") {
            self.motion.y = in_game_constant::VELOCITY * - 1.0;
            self.motion.x = 0.0;
            self.player_status = PlayerStatus::Walking 
        } 
        else if Input::is_action_pressed(&input, "Down") {
            self.motion.y = in_game_constant::VELOCITY;
            self.motion.x = 0.0;
            self.player_status = PlayerStatus::Walking 
        }
        else {
            self.motion.x = 0.0;
            self.motion.y = 0.0;
            self.player_status = PlayerStatus::Idle
        }
    }
}


#[gdnative::methods]
impl PlayerCharacter {  

    /// The `PlayerCharacter` constructor
    fn new(_owner: &KinematicBody2D) -> Self {
        Self {
            player_status: Default::default(),
            menu_status: MenuStatus::Closed,
            dialogue_box_status: DialogueBoxStatus::Inactive,
            motion: Vector2::new(0.0, 0.0),
            signals: HashMap::new(),
            current_position: Vector2::new(0.0, 0.0),
            counter: 0
        }
    }

    #[export]
    fn _ready(&mut self, owner: &KinematicBody2D) {
        
        // Retrieves the player absolute position from a JSON config file
        self.current_position.x = utils::get_player_absolute_position().0;
        self.current_position.y = utils::get_player_absolute_position().1;

        // Sets the retrieved position
        owner.set_global_position(Vector2::new(self.current_position.x, self.current_position.y));

        // Connect the Player Character with the Struct that takes care about process, manage and persist PlayerCharacter data
        self.connect_to_game_data(owner);
    }
    

    #[export]
    fn _physics_process(&mut self, owner: &KinematicBody2D, _delta: f32) {
        // First of all, we need a reference to our singleton(scene, node, value that exists through out the game) Input 
        let input: &Input = Input::godot_singleton();

        // All Y axis motions are affected first by the gravity
        // self.apply_gravity(&owner);
        
        // Calling the method who animates the sprite when the KinematicBody2D is moving
        self.animate_character(&owner);
        
        if self.player_status != PlayerStatus::Interacting {
            // Moving the player when an input is detected
            self.move_character(&owner, &input);
           
            // Saving a Ref after moves the `Player`, in case of collision, player movement will store the data about that collision
            let player_movement = owner.move_and_collide(
                self.motion * _delta, false, false, false);
            
            self.current_position = owner.global_position();
            self.counter += 1;
            // owner.set_global_position(self.current_position);

            // Check when the player press the `space bar` == "Interact" key binding. If the player isn't interacting with anything else
            // calls the `interact method`.
            if Input::is_action_just_pressed(&input, "Interact") {
                if self.player_status != PlayerStatus::Interacting {
                    self.interact(owner, player_movement);
                }
            }

            if Input::is_action_just_pressed(&input, "Menu") {
                owner.emit_signal("player_position", &[(self.current_position.x, self.current_position.y).to_variant()]);
            }
        }
    }

    /// Method designed to act as an intermediary when some event blocks any action of the player.
    ///
    /// Ex:
    /// The player talking with some other character is an interaction. While it's happening, the player
    /// should not be moving or doing anything else that "reading the Dialogue Box" with the text that the interaction has.
    ///
    /// The info parameter just provides an String that contains info from the signal that will be used to match
    /// a certain behaviour with that provided String.
    #[export]
    fn handle_interaction(&mut self, _owner: &KinematicBody2D, signal_info: String) {
        // Get a full `slice` of the parameters in order to match it with a `classical` &str
        let signal_info = &signal_info[..];
        
        // Matching the signal extra data
        match signal_info {
            "on_dialogue" => {
                self.player_status = PlayerStatus::Interacting;
                self.motion.x = 0.0;
                self.motion.y = 0.0;
                self.dialogue_box_status = DialogueBoxStatus::Active
            },
            "menu_active" => {
                self.player_status = PlayerStatus::Interacting;
                self.motion.x = 0.0;
                self.motion.y = 0.0;
                self.menu_status = MenuStatus::Open
            },
            _ => {
                self.player_status = PlayerStatus::default();
                self.dialogue_box_status = DialogueBoxStatus::Inactive;
                self.menu_status = MenuStatus::Closed
            }
        }
    }

    /// The method for the "Interaction" behaviour of the `Player Character`.
    ///
    /// Retrieves the Node which is colliding with our player character. 
    ///
    /// If there's Some() collision, checks if the object are allowed to interact with the player.
    /// Sends a signal alerting that the player if the object has an "Interact" child.
    fn interact(&mut self, owner: &KinematicBody2D, collision_data: Option<Ref<KinematicCollision2D>>) {
        match collision_data {
            Some(collision_data) => { 
                let collision: TRef<KinematicCollision2D, Shared> = unsafe { collision_data.assume_safe() }; 

                let coll_body: TRef<Node> = self.get_collision_body(collision);

                //  Notifies the game that the player is interacting if true
                if self.is_valid_interaction(coll_body) {
                    self.player_is_interacting(owner);
                }
            },
            _ => ()
        }
    }



    /// Send the "player interacting" custom signal, that alerts that the player is currently on `PlayerStatus::Interacting` state.
    fn player_is_interacting(&self, owner: &KinematicBody2D) {
        owner.emit_signal("player_interacting", &[]);
    }

    /// Given a body that is colliding with the `Player Character`, checks if has an "Interaction" Node,
    /// that represents that the object holds data for the player, and the `PlayerStatus`, which has to currently be == `PlayerStatus::Interacting`
    ///
    /// If the required conditions are satisfied, returns true.
    /// 
    /// Remember that in Rust, `if` expressions without `else` evaluate to `()`
    fn is_valid_interaction(&self, coll_body: TRef<Node>) -> bool {
        if coll_body.has_node("Interact") && self.dialogue_box_status == DialogueBoxStatus::Inactive { 
            return true; 
        } else { return false; }
    }

    /// Returns a TRef<Node> of the body that is colliding with our player
    fn get_collision_body(&self, collision: TRef<KinematicCollision2D, Shared>) -> TRef<Node> {
        unsafe { collision
            .collider()
            .unwrap()
            .assume_safe()
          }.cast::<Node>().unwrap()
    }

    /// If the player character is moving, should be an animated representation.
    ///
    /// Emit the signal "animate" and send the current player motion data for the receivers 
    fn animate_character(&self, owner: &KinematicBody2D) {
        owner.emit_signal("animate", &[self.motion.to_variant()]);
    }

    /// Connects the PlayerCharacter signal that transmits the current global position
    fn connect_to_game_data(&self, owner: &KinematicBody2D) {
        let receiver = unsafe { owner.get_node("/root/Game").unwrap().assume_safe() };
        owner.connect("player_position", receiver,
         "_save_player_position", VariantArray::new_shared(), 0).unwrap();
    }

}

#[derive(NativeClass)]
#[inherit(AnimatedSprite)]
#[register_with(Self::register_signal)]
#[derive(Debug)]
pub struct PlayerAnimation {
    current_player_motion: PlayerStatus,
    current_player_direction: PlayerDirection,
    idle_player_direction: PlayerDirection
}

impl RegisterSignal<Self> for PlayerAnimation {
    fn register_signal(builder: &ClassBuilder<Self>) {
        // Indicates that the Player is moving
        builder.add_signal( Signal {
            name: "player_direction",
            args: &[],
        });
    }
}

#[gdnative::methods]
impl PlayerAnimation {
    fn new(_owner: &AnimatedSprite) -> Self {
        Self {
            current_player_motion: Default::default(),
            current_player_direction: Default::default(),
            idle_player_direction: Default::default()
        }
    }

    #[export]
    fn _ready(&mut self, owner: &AnimatedSprite) {
        owner.set_process(true);
        self.idle_player_direction = utils::get_player_direction();
        match self.idle_player_direction {
            PlayerDirection::Downwards => { owner.play("idle front", false); }
            PlayerDirection::Upwards => { owner.play("idle back", false); }
            PlayerDirection::Left => { owner.play("idle left", false); }
            PlayerDirection::Right => { owner.play("idle right", false); }
        }; 

        // Connects with the Game class
        self.connect_to_game_data(owner);
    }

    #[export]
    fn _process(&self, owner: &AnimatedSprite, _delta: f64) {
        let input: &Input = Input::godot_singleton();
        if Input::is_action_just_pressed(&input, "Menu") {
            owner.emit_signal("player_direction", &[self.idle_player_direction.to_variant()]);
        }
    }

    #[export]
    fn _on_player_animate(&mut self, _owner: &AnimatedSprite, _motion: Vector2) {
        
        let character_animated_sprite = unsafe { _owner.get_node_as::<AnimatedSprite>( ".") }.unwrap();

        match _motion {
            x if x.x > 0.0 => 
                { self.current_player_direction = PlayerDirection::Right; self.current_player_motion = PlayerStatus::Walking },

            x if x.x < 0.0 => 
                { self.current_player_direction = PlayerDirection::Left; self.current_player_motion = PlayerStatus::Walking }, 

            x if x.y < 0.0 => 
                { self.current_player_direction = PlayerDirection::Upwards; self.current_player_motion = PlayerStatus::Walking },
            
            x if x.y > 0.0 => 
                { self.current_player_direction = PlayerDirection::Downwards; self.current_player_motion = PlayerStatus::Walking },
            
            _ => 
                { self.current_player_motion = PlayerStatus::Idle }
                
        }


        if self.current_player_motion == PlayerStatus::Idle {
            match self.idle_player_direction {
                PlayerDirection::Downwards => { character_animated_sprite.play("idle front", false); }
                PlayerDirection::Upwards => { character_animated_sprite.play("idle back", false); }
                PlayerDirection::Left => { character_animated_sprite.play("idle left", false); }
                PlayerDirection::Right => { character_animated_sprite.play("idle right", false); }
            }; 

        } else if self.current_player_direction == PlayerDirection::Right {
            character_animated_sprite.play("walk right", false);
            self.idle_player_direction = PlayerDirection::Right;

        } else if PlayerDirection::Left == self.current_player_direction {
            character_animated_sprite.play("walk left", false);
            self.idle_player_direction = PlayerDirection::Left;

        } else if PlayerDirection::Downwards == self.current_player_direction {
            character_animated_sprite.play("walk downwards", false);
            self.idle_player_direction = PlayerDirection::Downwards;

        } else if PlayerDirection::Upwards == self.current_player_direction {
            character_animated_sprite.play("walk upwards", false);
            self.idle_player_direction = PlayerDirection::Upwards;
        }

    }

    /// Connects the PlayerCharacter signal that transmits the PC position
    fn connect_to_game_data(&self, owner: &AnimatedSprite) {
        let receiver = unsafe { owner.get_node("/root/Game").unwrap().assume_safe() };
        owner.connect("player_direction", receiver,
            "_save_player_direction", VariantArray::new_shared(), 0).unwrap();
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum PlayerStatus {
    Idle,
    Walking,
    // Running
    Interacting
}

impl Default for PlayerStatus {
    fn default() -> Self { PlayerStatus::Idle }
}

#[derive(PartialEq, Clone, Debug, ToVariant, Deserialize)]
pub enum PlayerDirection {
    Upwards,
    Downwards,
    Left,
    Right
}

impl Default for PlayerDirection {
    fn default() -> Self { PlayerDirection::Downwards }
}

impl Serialize for PlayerDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            PlayerDirection::Upwards => serializer.serialize_unit_variant("PlayerDirection", 0, "Upwards"),
            PlayerDirection::Downwards => serializer.serialize_unit_variant("PlayerDirection", 1, "Downwards"),
            PlayerDirection::Left => serializer.serialize_unit_variant("PlayerDirection", 2, "Left"),
            PlayerDirection::Right => serializer.serialize_unit_variant("PlayerDirection", 3, "Right"),
        }
    }
}