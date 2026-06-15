// PATHFINDER Achievement Service
// Badges, achievements, gamification, and goal tracking
// Port: 8008

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"github.com/rs/cors"
)

var db *sql.DB

// ============================================================================
// DATA MODELS
// ============================================================================

type Achievement struct {
	ID          string    `json:"id"`
	UserID      string    `json:"user_id"`
	BadgeID     string    `json:"badge_id"`
	BadgeName   string    `json:"badge_name"`
	Category    string    `json:"category"` // skill_mastery, streak, speed, accuracy
	Description string    `json:"description"`
	IconURL     string    `json:"icon_url"`
	UnlockedAt  time.Time `json:"unlocked_at"`
	CreatedAt   time.Time `json:"created_at"`
}

type Badge struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Category    string `json:"category"`
	Description string `json:"description"`
	IconURL     string `json:"icon_url"`
	Requirement string `json:"requirement"` // JSON: {type: "mastery_count", value: 10}
	Rarity      string `json:"rarity"` // common, uncommon, rare, epic, legendary
	Points      int    `json:"points"`
	CreatedAt   time.Time `json:"created_at"`
}

type Goal struct {
	ID          string     `json:"id"`
	UserID      string     `json:"user_id"`
	Title       string     `json:"title"`
	Description string     `json:"description"`
	Type        string     `json:"type"` // skills_to_master, accuracy_target, streak_target
	Target      int        `json:"target"`
	Current     int        `json:"current"`
	Deadline    time.Time  `json:"deadline"`
	Status      string     `json:"status"` // active, completed, failed
	CompletedAt *time.Time `json:"completed_at,omitempty"`
	CreatedAt   time.Time  `json:"created_at"`
}

type LeaderboardEntry struct {
	UserID       string `json:"user_id"`
	UserName     string `json:"user_name"`
	Rank         int    `json:"rank"`
	Points       int    `json:"points"`
	Achievements int    `json:"achievements"`
	Mastery      float64 `json:"mastery_percent"`
	Streak       int    `json:"streak_days"`
}

type GamificationStats struct {
	UserID       string `json:"user_id"`
	TotalPoints  int    `json:"total_points"`
	Achievements int    `json:"achievements_count"`
	Badges       int    `json:"badges_unlocked"`
	Level        int    `json:"level"`
	NextLevelXP  int    `json:"next_level_xp"`
	Rank         int    `json:"leaderboard_rank"`
}

// ============================================================================
// ACHIEVEMENT ENDPOINTS
// ============================================================================

// GET /v1/achievements - Get user's achievements
func getAchievements(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT a.id, a.user_id, a.badge_id, b.name, b.category, b.description,
	       b.icon_url, a.unlocked_at, a.created_at
	FROM achievements a
	JOIN badges b ON a.badge_id = b.id
	WHERE a.user_id = $1
	ORDER BY a.unlocked_at DESC
	`

	rows, err := db.Query(query, userID)
	if err != nil {
		log.Printf("Error getting achievements: %v\n", err)
		http.Error(w, "Failed to get achievements", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var achievements []Achievement
	for rows.Next() {
		var a Achievement
		err := rows.Scan(
			&a.ID, &a.UserID, &a.BadgeID, &a.BadgeName, &a.Category,
			&a.Description, &a.IconURL, &a.UnlockedAt, &a.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning achievement: %v\n", err)
			continue
		}
		achievements = append(achievements, a)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"achievements": achievements,
		"count":        len(achievements),
	})
}

// POST /v1/achievements/unlock - Unlock achievement (internal)
func unlockAchievement(w http.ResponseWriter, r *http.Request) {
	var req struct {
		UserID  string `json:"user_id"`
		BadgeID string `json:"badge_id"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Verify this is an internal call (from another service)
	// In production, use service-to-service auth (JWT, mTLS, etc.)

	achievementID := uuid.New().String()
	now := time.Now().UTC()

	// Check if already unlocked
	var exists bool
	err := db.QueryRow(
		"SELECT EXISTS(SELECT 1 FROM achievements WHERE user_id = $1 AND badge_id = $2)",
		req.UserID, req.BadgeID,
	).Scan(&exists)

	if err != nil {
		log.Printf("Error checking achievement: %v\n", err)
		http.Error(w, "Failed to check achievement", http.StatusInternalServerError)
		return
	}

	if exists {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"status": "already_unlocked",
		})
		return
	}

	// Unlock achievement
	_, err = db.Exec(`
		INSERT INTO achievements (id, user_id, badge_id, unlocked_at, created_at)
		VALUES ($1, $2, $3, $4, $5)
	`,
		achievementID, req.UserID, req.BadgeID, now, now,
	)

	if err != nil {
		log.Printf("Error unlocking achievement: %v\n", err)
		http.Error(w, "Failed to unlock achievement", http.StatusInternalServerError)
		return
	}

	// Award points
	var points int
	err = db.QueryRow(
		"SELECT points FROM badges WHERE id = $1",
		req.BadgeID,
	).Scan(&points)

	if err == nil {
		_, _ = db.Exec(
			"UPDATE user_gamification SET total_points = total_points + $1 WHERE user_id = $2",
			points, req.UserID,
		)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"id":     achievementID,
		"status": "unlocked",
	})
}

// ============================================================================
// BADGE ENDPOINTS
// ============================================================================

// GET /v1/badges - Get all available badges
func getBadges(w http.ResponseWriter, r *http.Request) {
	category := r.URL.Query().Get("category")

	var query string
	var args []interface{}

	if category != "" {
		query = "SELECT id, name, category, description, icon_url, requirement, rarity, points, created_at FROM badges WHERE category = $1 ORDER BY rarity, created_at"
		args = append(args, category)
	} else {
		query = "SELECT id, name, category, description, icon_url, requirement, rarity, points, created_at FROM badges ORDER BY rarity, created_at"
	}

	rows, err := db.Query(query, args...)
	if err != nil {
		log.Printf("Error getting badges: %v\n", err)
		http.Error(w, "Failed to get badges", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var badges []Badge
	for rows.Next() {
		var b Badge
		err := rows.Scan(
			&b.ID, &b.Name, &b.Category, &b.Description,
			&b.IconURL, &b.Requirement, &b.Rarity, &b.Points, &b.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning badge: %v\n", err)
			continue
		}
		badges = append(badges, b)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"badges": badges,
		"count":  len(badges),
	})
}

// ============================================================================
// GOAL ENDPOINTS
// ============================================================================

// POST /v1/goals - Create goal
func createGoal(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var req Goal
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	goalID := uuid.New().String()
	now := time.Now().UTC()

	_, err := db.Exec(`
		INSERT INTO goals (id, user_id, title, description, type, target, current, deadline, status, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
	`,
		goalID, userID, req.Title, req.Description, req.Type,
		req.Target, 0, req.Deadline, "active", now,
	)

	if err != nil {
		log.Printf("Error creating goal: %v\n", err)
		http.Error(w, "Failed to create goal", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"id":     goalID,
		"status": "created",
	})
}

// GET /v1/goals - Get user's goals
func getGoals(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	status := r.URL.Query().Get("status") // all, active, completed

	var query string
	var args []interface{}

	query = "SELECT id, user_id, title, description, type, target, current, deadline, status, completed_at, created_at FROM goals WHERE user_id = $1"
	args = append(args, userID)

	if status != "" && status != "all" {
		query += " AND status = $2"
		args = append(args, status)
	}

	query += " ORDER BY created_at DESC"

	rows, err := db.Query(query, args...)
	if err != nil {
		log.Printf("Error getting goals: %v\n", err)
		http.Error(w, "Failed to get goals", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var goals []Goal
	for rows.Next() {
		var g Goal
		err := rows.Scan(
			&g.ID, &g.UserID, &g.Title, &g.Description, &g.Type,
			&g.Target, &g.Current, &g.Deadline, &g.Status, &g.CompletedAt, &g.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning goal: %v\n", err)
			continue
		}
		goals = append(goals, g)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"goals": goals,
		"count": len(goals),
	})
}

// PUT /v1/goals/:id - Update goal progress
func updateGoalProgress(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	goalID := r.URL.Query().Get("id")

	if userID == "" || goalID == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	var req struct {
		Current int `json:"current"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Verify ownership
	var id string
	var target int
	err := db.QueryRow(
		"SELECT id, target FROM goals WHERE id = $1 AND user_id = $2",
		goalID, userID,
	).Scan(&id, &target)

	if err == sql.ErrNoRows {
		http.Error(w, "Goal not found", http.StatusNotFound)
		return
	}

	// Update progress
	status := "active"
	var completedAt *time.Time
	if req.Current >= target {
		status = "completed"
		now := time.Now().UTC()
		completedAt = &now
	}

	_, err = db.Exec(
		"UPDATE goals SET current = $1, status = $2, completed_at = $3 WHERE id = $4",
		req.Current, status, completedAt, goalID,
	)

	if err != nil {
		log.Printf("Error updating goal: %v\n", err)
		http.Error(w, "Failed to update goal", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"status": "updated",
	})
}

// DELETE /v1/goals/:id - Delete goal
func deleteGoal(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	goalID := r.URL.Query().Get("id")

	if userID == "" || goalID == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	result, err := db.Exec(
		"DELETE FROM goals WHERE id = $1 AND user_id = $2",
		goalID, userID,
	)

	if err != nil {
		log.Printf("Error deleting goal: %v\n", err)
		http.Error(w, "Failed to delete goal", http.StatusInternalServerError)
		return
	}

	rows, _ := result.RowsAffected()
	if rows == 0 {
		http.Error(w, "Goal not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "deleted",
	})
}

// ============================================================================
// LEADERBOARD ENDPOINTS
// ============================================================================

// GET /v1/leaderboard - Get global leaderboard
func getLeaderboard(w http.ResponseWriter, r *http.Request) {
	limit := 100
	offset := 0

	if l := r.URL.Query().Get("limit"); l != "" {
		fmt.Sscanf(l, "%d", &limit)
	}
	if o := r.URL.Query().Get("offset"); o != "" {
		fmt.Sscanf(o, "%d", &offset)
	}

	query := `
	SELECT ROW_NUMBER() OVER (ORDER BY ug.total_points DESC) as rank,
	       u.id, CONCAT(u.first_name, ' ', u.last_name),
	       ug.total_points, COUNT(DISTINCT a.id) as achievements,
	       COALESCE(lp.mastery_percentage * 100, 0),
	       COALESCE(lp.current_streak, 0)
	FROM user_gamification ug
	JOIN users u ON ug.user_id = u.id
	LEFT JOIN achievements a ON ug.user_id = a.user_id
	LEFT JOIN learner_progress lp ON ug.user_id = lp.user_id
	GROUP BY ug.user_id, u.id, u.first_name, u.last_name, ug.total_points, lp.mastery_percentage, lp.current_streak
	ORDER BY ug.total_points DESC
	LIMIT $1 OFFSET $2
	`

	rows, err := db.Query(query, limit, offset)
	if err != nil {
		log.Printf("Error getting leaderboard: %v\n", err)
		http.Error(w, "Failed to get leaderboard", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var entries []LeaderboardEntry
	for rows.Next() {
		var e LeaderboardEntry
		err := rows.Scan(
			&e.Rank, &e.UserID, &e.UserName,
			&e.Points, &e.Achievements, &e.Mastery, &e.Streak,
		)
		if err != nil {
			log.Printf("Error scanning leaderboard: %v\n", err)
			continue
		}
		entries = append(entries, e)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"leaderboard": entries,
		"count":       len(entries),
	})
}

// GET /v1/leaderboard/rank - Get user's rank
func getUserRank(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT ROW_NUMBER() OVER (ORDER BY ug.total_points DESC)
	FROM user_gamification ug
	WHERE ug.user_id = $1
	`

	var rank int
	err := db.QueryRow(query, userID).Scan(&rank)

	if err == sql.ErrNoRows {
		rank = 0
	} else if err != nil {
		log.Printf("Error getting rank: %v\n", err)
		http.Error(w, "Failed to get rank", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"rank": rank,
	})
}

// ============================================================================
// GAMIFICATION STATS ENDPOINTS
// ============================================================================

// GET /v1/gamification/stats - Get user's gamification stats
func getGamificationStats(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT ug.user_id, ug.total_points,
	       COALESCE(COUNT(DISTINCT a.id), 0) as achievements,
	       COALESCE(COUNT(DISTINCT b.id), 0) as badges,
	       FLOOR(ug.total_points / 100) + 1 as level,
	       ((ug.total_points % 100) / 100.0 * 100)::int as progress_to_next,
	       ROW_NUMBER() OVER (ORDER BY ug.total_points DESC)
	FROM user_gamification ug
	LEFT JOIN achievements a ON ug.user_id = a.user_id
	LEFT JOIN badges b ON EXISTS(SELECT 1 FROM achievements WHERE user_id = ug.user_id AND badge_id = b.id)
	WHERE ug.user_id = $1
	GROUP BY ug.user_id, ug.total_points
	`

	var stats GamificationStats
	var nextLevelXP int

	err := db.QueryRow(query, userID).Scan(
		&stats.UserID, &stats.TotalPoints, &stats.Achievements,
		&stats.Badges, &stats.Level, &nextLevelXP, &stats.Rank,
	)

	if err == sql.ErrNoRows {
		// Initialize gamification stats
		stats.UserID = userID
		stats.TotalPoints = 0
		stats.Achievements = 0
		stats.Badges = 0
		stats.Level = 1
		stats.NextLevelXP = 100
		stats.Rank = 0

		_, _ = db.Exec(
			"INSERT INTO user_gamification (user_id, total_points, created_at) VALUES ($1, $2, $3)",
			userID, 0, time.Now().UTC(),
		)
	} else if err != nil {
		log.Printf("Error getting gamification stats: %v\n", err)
		http.Error(w, "Failed to get stats", http.StatusInternalServerError)
		return
	}

	stats.NextLevelXP = (stats.Level * 100) - stats.TotalPoints%100

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// ============================================================================
// MAIN
// ============================================================================

func main() {
	// Connect to database
	dbURL := os.Getenv("DATABASE_URL")
	if dbURL == "" {
		dbURL = "postgres://postgres:postgres@localhost/pathfinder"
	}

	var err error
	db, err = sql.Open("postgres", dbURL)
	if err != nil {
		log.Fatalf("Failed to connect to database: %v\n", err)
	}
	defer db.Close()

	// Setup router
	r := mux.NewRouter()

	// Achievement endpoints
	r.HandleFunc("/v1/achievements", getAchievements).Methods("GET")
	r.HandleFunc("/v1/achievements/unlock", unlockAchievement).Methods("POST")

	// Badge endpoints
	r.HandleFunc("/v1/badges", getBadges).Methods("GET")

	// Goal endpoints
	r.HandleFunc("/v1/goals", createGoal).Methods("POST")
	r.HandleFunc("/v1/goals", getGoals).Methods("GET")
	r.HandleFunc("/v1/goals/update", updateGoalProgress).Methods("PUT")
	r.HandleFunc("/v1/goals/delete", deleteGoal).Methods("DELETE")

	// Leaderboard endpoints
	r.HandleFunc("/v1/leaderboard", getLeaderboard).Methods("GET")
	r.HandleFunc("/v1/leaderboard/rank", getUserRank).Methods("GET")

	// Gamification endpoints
	r.HandleFunc("/v1/gamification/stats", getGamificationStats).Methods("GET")

	// CORS
	c := cors.Default()
	handler := c.Handler(r)

	// Start server
	port := os.Getenv("ACHIEVEMENT_SERVICE_PORT")
	if port == "" {
		port = "8008"
	}

	log.Printf("Achievement Service listening on port %s\n", port)
	log.Fatal(http.ListenAndServe(":"+port, handler))
}
