# Sylva Pong – Terminal-based playable Pong game

def new_game()
    {
        ball_x: 40,
        ball_y: 12,
        ball_dx: 1,
        ball_dy: 1,
        paddle1_y: 10,
        paddle2_y: 10,
        score1: 0,
        score2: 0,
        running: true
    }

def clamp(val, lo, hi)
    if val < lo
        lo
    else
        if val > hi
            hi
        else
            val

def update_game(game, up1, down1, up2, down2)
    let paddle1 = clamp(game["paddle1_y"] + (if up1 then -1 else (if down1 then 1 else 0)), 0, 20)
    let paddle2 = clamp(game["paddle2_y"] + (if up2 then -1 else (if down2 then 1 else 0)), 0, 20)
    let ball_x = game["ball_x"] + game["ball_dx"]
    let ball_y = game["ball_y"] + game["ball_dy"]
    let ball_dy_new = if ball_y <= 0 or ball_y >= 23 then -(game["ball_dy"]) else game["ball_dy"]
    let ball_dx_new = if ball_x <= 2 and ball_y >= paddle1 and ball_y <= paddle1 + 4 then -(game["ball_dx"]) else game["ball_dx"]
    let ball_dx_new2 = if ball_x >= 77 and ball_y >= paddle2 and ball_y <= paddle2 + 4 then -ball_dx_new else ball_dx_new
    let score1_new = if ball_x <= 0 then game["score1"] + 1 else game["score1"]
    let score2_new = if ball_x >= 79 then game["score2"] + 1 else game["score2"]
    let reset_ball = if ball_x <= 0 or ball_x >= 79 then true else false
    let ball_x_final = if reset_ball then 40 else ball_x
    let ball_y_final = if reset_ball then 12 else ball_y
    let ball_dx_final = if reset_ball then 1 else ball_dx_new2
    let ball_dy_final = if reset_ball then 1 else ball_dy_new
    {
        ball_x: ball_x_final,
        ball_y: ball_y_final,
        ball_dx: ball_dx_final,
        ball_dy: ball_dy_final,
        paddle1_y: paddle1,
        paddle2_y: paddle2,
        score1: score1_new,
        score2: score2_new,
        running: true
    }

def draw_game(game)
    print("Score: " + str(game["score1"]) + " - " + str(game["score2"]))

def play()
    let game = new_game()
    while game["running"]
        draw_game(game)
        let up1 = false
        let down1 = false
        let up2 = false
        let down2 = false
        print("w/s=left, o/l=right, q=quit")
        let ch = input("> ")
        let up1 = if ch == "w" then true else false
        let down1 = if ch == "s" then true else false
        let up2 = if ch == "o" then true else false
        let down2 = if ch == "l" then true else false
        if ch == "q"
            return nil
        else
            nil
        game = update_game(game, up1, down1, up2, down2)
        sleep(0.05)

play()
