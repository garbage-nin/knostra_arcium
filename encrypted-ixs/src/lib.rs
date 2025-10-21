use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct InputValues {
        v1: u8,
        v2: u8,
    }

    pub struct GameMoves {
        pub yes_cards1: u8,
        pub yes_cards2: u8,
        pub yes_cards3: u8,
        pub no_cards1: u8,
        pub no_cards2: u8,
        pub no_cards3: u8,
    }

    pub struct PlayerJoin {      // 0 = yes, 1 = no
        pub player_cards1: u8,      // encrypted card id
        pub player_cards2: u8,      // encrypted card id
        pub player_cards3: u8,      // encrypted card id
    }
    #[instruction]
    pub fn add_together(input_ctxt: Enc<Shared, InputValues>) -> Enc<Shared, u16> {
        let input = input_ctxt.to_arcis();
        let sum = input.v1 as u16 + input.v2 as u16;
        input_ctxt.owner.from_arcis(sum)
    }

    #[instruction]
    pub fn init_game(mxe: Mxe) -> Enc<Mxe, GameMoves> {
        let game_moves = GameMoves {
            yes_cards1: 0,
            yes_cards2: 0,
            yes_cards3: 0,
            no_cards1: 0,
            no_cards2: 0,
            no_cards3: 0,
        };

        // Encrypt the initial state for Arcium
        mxe.from_arcis(game_moves)
    }

    #[instruction]
    pub fn join_game(
        player_join_ctxt: Enc<Shared, PlayerJoin>,
        game_ctxt: Enc<Mxe, GameMoves>,        
        player_side: u8,                      
    ) -> Enc<Mxe, GameMoves> {
        let player_join = player_join_ctxt.to_arcis();
        let mut game_state = game_ctxt.to_arcis();

    if player_side == 0 {
        if game_state.yes_cards1 == 0
            && game_state.yes_cards2 == 0
            && game_state.yes_cards3 == 0
        {
            game_state.yes_cards1 = player_join.player_cards1;
            game_state.yes_cards2 = player_join.player_cards2;
            game_state.yes_cards3 = player_join.player_cards3;
        }
    } else {
        if game_state.no_cards1 == 0
            && game_state.no_cards2 == 0
            && game_state.no_cards3 == 0
        {
            game_state.no_cards1 = player_join.player_cards1;
            game_state.no_cards2 = player_join.player_cards2;
            game_state.no_cards3 = player_join.player_cards3;
        }
    }

        // Re-encrypt the updated game state
        game_ctxt.owner.from_arcis(game_state)
    }

}
