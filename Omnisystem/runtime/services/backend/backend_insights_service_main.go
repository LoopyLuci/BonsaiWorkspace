// PATHFINDER Learning Insights Service
// Personalized recommendations, study planning, and learning analytics
// Port: 8009

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"math"
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

type LearningAnalytics struct {
	UserID               string    `json:"user_id"`
	TotalSkills          int       `json:"total_skills"`
	MasteredSkills       int       `json:"mastered_skills"`
	StruggleSkills       int       `json:"struggle_skills"`
	AverageMastery       float64   `json:"average_mastery"`
	AverageAccuracy      float64   `json:"average_accuracy"`
	TotalExercises       int       `json:"total_exercises"`
	TotalTimeSpent       int       `json:"total_time_spent_minutes"`
	AverageTimePerEx     int       `json:"average_time_per_exercise"`
	LongestStreak        int       `json:"longest_streak"`
	CurrentStreak        int       `json:"current_streak"`
	LastActivityTime     time.Time `json:"last_activity_time"`
	AverageSessionLength int       `json:"average_session_length"`
}

type Recommendation struct {
	ID          string `json:"id"`
	UserID      string `json:"user_id"`
	Type        string `json:"type"` // practice, review, learn, rest
	SkillName   string `json:"skill_name"`
	Reason      string `json:"reason"`
	Priority    string `json:"priority"` // high, medium, low
	ActionText  string `json:"action_text"`
	TimeNeeded  int    `json:"time_needed_minutes"`
	DueDate     time.Time `json:"due_date"`
	CreatedAt   time.Time `json:"created_at"`
}

type StudySession struct {
	ID          string `json:"id"`
	UserID      string `json:"user_id"`
	SkillName   string `json:"skill_name"`
	Duration    int    `json:"duration_minutes"`
	Difficulty  string `json:"difficulty"` // easy, medium, hard
	ScheduledFor time.Time `json:"scheduled_for"`
	Status      string `json:"status"` // scheduled, in_progress, completed, skipped
	CompletedAt *time.Time `json:"completed_at,omitempty"`
	CreatedAt   time.Time `json:"created_at"`
}

type LearningStyle struct {
	UserID       string `json:"user_id"`
	Visual       float64 `json:"visual_percent"`      // Charts, diagrams, visual content
	Auditory     float64 `json:"auditory_percent"`    // Videos, audio, discussions
	Kinesthetic  float64 `json:"kinesthetic_percent"` // Hands-on, interactive
	Reading      float64 `json:"reading_percent"`     // Text, books, written content
	DominantStyle string `json:"dominant_style"`
	UpdatedAt    time.Time `json:"updated_at"`
}

type PerformanceMetric struct {
	SkillName        string    `json:"skill_name"`
	MasteryPercent   float64   `json:"mastery_percent"`
	AccuracyPercent  float64   `json:"accuracy_percent"`
	TimeToMastery    int       `json:"time_to_mastery_hours"`
	ExercisesAttempted int    `json:"exercises_attempted"`
	ExercisesCorrect int       `json:"exercises_correct"`
	Trend            string    `json:"trend"` // up, stable, down
	PredictedMastery float64   `json:"predicted_mastery_percent"`
}

// ============================================================================
// LEARNING ANALYTICS ENDPOINTS
// ============================================================================

// GET /v1/insights/analytics - Get user's learning analytics
func getLearningAnalytics(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT COUNT(DISTINCT s.id) as total_skills,
	       COUNT(DISTINCT CASE WHEN lp.mastery_percentage >= 0.85 THEN s.id END) as mastered,
	       COUNT(DISTINCT CASE WHEN lp.mastery_percentage < 0.3 THEN s.id END) as struggling,
	       COALESCE(AVG(lp.mastery_percentage) * 100, 0) as avg_mastery,
	       COALESCE(SUM(CASE WHEN ea.was_correct THEN 1 ELSE 0 END)::float / NULLIF(COUNT(ea), 0) * 100, 0) as avg_accuracy,
	       COUNT(ea.id) as total_exercises,
	       COALESCE(SUM(EXTRACT(EPOCH FROM (ea.completed_at - ea.created_at)) / 60), 0)::int as total_time,
	       COALESCE(AVG(EXTRACT(EPOCH FROM (ea.completed_at - ea.created_at)) / 60), 0)::int as avg_time_per_ex,
	       COALESCE(lp.longest_streak, 0) as longest_streak,
	       COALESCE(lp.current_streak, 0) as current_streak,
	       COALESCE(MAX(ea.created_at), NOW()) as last_activity,
	       0 as avg_session_length
	FROM users u
	LEFT JOIN learner_progress lp ON u.id = lp.user_id
	LEFT JOIN skills s ON lp.skill_id = s.id
	LEFT JOIN exercise_attempts ea ON u.id = ea.user_id
	WHERE u.id = $1
	GROUP BY u.id, lp.longest_streak, lp.current_streak
	`

	var analytics LearningAnalytics
	analytics.UserID = userID

	err := db.QueryRow(query, userID).Scan(
		&analytics.TotalSkills, &analytics.MasteredSkills, &analytics.StruggleSkills,
		&analytics.AverageMastery, &analytics.AverageAccuracy, &analytics.TotalExercises,
		&analytics.TotalTimeSpent, &analytics.AverageTimePerEx,
		&analytics.LongestStreak, &analytics.CurrentStreak, &analytics.LastActivityTime,
		&analytics.AverageSessionLength,
	)

	if err != nil && err != sql.ErrNoRows {
		log.Printf("Error getting analytics: %v\n", err)
		http.Error(w, "Failed to get analytics", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(analytics)
}

// GET /v1/insights/recommendations - Get personalized recommendations
func getRecommendations(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	limit := 10
	if l := r.URL.Query().Get("limit"); l != "" {
		fmt.Sscanf(l, "%d", &limit)
	}

	query := `
	SELECT ir.id, ir.user_id, ir.type, ir.skill_name, ir.reason,
	       ir.priority, ir.action_text, ir.time_needed, ir.due_date, ir.created_at
	FROM insight_recommendations ir
	WHERE ir.user_id = $1 AND ir.created_at > NOW() - INTERVAL '7 days'
	ORDER BY CASE ir.priority WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END,
	         ir.created_at DESC
	LIMIT $2
	`

	rows, err := db.Query(query, userID, limit)
	if err != nil {
		log.Printf("Error getting recommendations: %v\n", err)
		http.Error(w, "Failed to get recommendations", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var recommendations []Recommendation
	for rows.Next() {
		var r Recommendation
		err := rows.Scan(
			&r.ID, &r.UserID, &r.Type, &r.SkillName, &r.Reason,
			&r.Priority, &r.ActionText, &r.TimeNeeded, &r.DueDate, &r.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning recommendation: %v\n", err)
			continue
		}
		recommendations = append(recommendations, r)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"recommendations": recommendations,
		"count":           len(recommendations),
	})
}

// POST /v1/insights/recommendations/generate - Generate recommendations (internal)
func generateRecommendations(w http.ResponseWriter, r *http.Request) {
	var req struct {
		UserID string `json:"user_id"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Get user's learning data
	var avgMastery, avgAccuracy float64
	var daysInactive int

	err := db.QueryRow(`
		SELECT COALESCE(AVG(lp.mastery_percentage) * 100, 0),
		       COALESCE(SUM(CASE WHEN ea.was_correct THEN 1 ELSE 0 END)::float / NULLIF(COUNT(ea), 0) * 100, 0),
		       COALESCE(EXTRACT(DAY FROM (NOW() - MAX(ea.created_at))), 0)::int
		FROM learner_progress lp
		LEFT JOIN exercise_attempts ea ON lp.user_id = ea.user_id
		WHERE lp.user_id = $1
	`, req.UserID).Scan(&avgMastery, &avgAccuracy, &daysInactive)

	if err != nil && err != sql.ErrNoRows {
		log.Printf("Error getting user data: %v\n", err)
		http.Error(w, "Failed to generate recommendations", http.StatusInternalServerError)
		return
	}

	// Generate recommendations based on patterns
	var recommendations []map[string]interface{}
	now := time.Now().UTC()

	// Recommendation 1: Review struggling skills
	if daysInactive > 3 {
		recommendations = append(recommendations, map[string]interface{}{
			"type":       "review",
			"priority":   "high",
			"reason":     "You haven't practiced in " + fmt.Sprintf("%d days", daysInactive),
			"action":     "Review your most difficult skills",
			"time_needed": 30,
		})
	}

	// Recommendation 2: Practice for improvement
	if avgMastery < 85 && avgAccuracy > 70 {
		recommendations = append(recommendations, map[string]interface{}{
			"type":        "practice",
			"priority":    "medium",
			"reason":      "You're close to mastery on some skills",
			"action":      "Practice high-potential skills",
			"time_needed": 20,
		})
	}

	// Recommendation 3: Rest/break
	if daysInactive == 0 {
		recommendations = append(recommendations, map[string]interface{}{
			"type":        "rest",
			"priority":    "low",
			"reason":      "You've been learning consistently",
			"action":      "Take a break and come back refreshed",
			"time_needed": 0,
		})
	}

	// Store recommendations
	for _, rec := range recommendations {
		recID := uuid.New().String()
		_, _ = db.Exec(`
			INSERT INTO insight_recommendations (id, user_id, type, reason, priority, action_text, time_needed, due_date, created_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		`,
			recID, req.UserID, rec["type"], rec["reason"], rec["priority"],
			rec["action"], rec["time_needed"], now.AddDate(0, 0, 1), now,
		)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"status":            "generated",
		"recommendations":   len(recommendations),
	})
}

// ============================================================================
// STUDY PLANNING ENDPOINTS
// ============================================================================

// POST /v1/insights/study-plan - Create study session
func createStudySession(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var req StudySession
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	sessionID := uuid.New().String()
	now := time.Now().UTC()

	_, err := db.Exec(`
		INSERT INTO study_sessions (id, user_id, skill_name, duration, difficulty, scheduled_for, status, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
	`,
		sessionID, userID, req.SkillName, req.Duration, req.Difficulty, req.ScheduledFor, "scheduled", now,
	)

	if err != nil {
		log.Printf("Error creating study session: %v\n", err)
		http.Error(w, "Failed to create session", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"id":     sessionID,
		"status": "created",
	})
}

// GET /v1/insights/study-plan - Get study plan
func getStudyPlan(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	status := r.URL.Query().Get("status") // all, scheduled, completed

	var query string
	var args []interface{}

	query = `
	SELECT id, user_id, skill_name, duration, difficulty, scheduled_for, status, completed_at, created_at
	FROM study_sessions
	WHERE user_id = $1
	`
	args = append(args, userID)

	if status != "" && status != "all" {
		query += " AND status = $2"
		args = append(args, status)
	}

	query += " ORDER BY scheduled_for ASC"

	rows, err := db.Query(query, args...)
	if err != nil {
		log.Printf("Error getting study plan: %v\n", err)
		http.Error(w, "Failed to get plan", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var sessions []StudySession
	for rows.Next() {
		var s StudySession
		err := rows.Scan(
			&s.ID, &s.UserID, &s.SkillName, &s.Duration, &s.Difficulty,
			&s.ScheduledFor, &s.Status, &s.CompletedAt, &s.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning session: %v\n", err)
			continue
		}
		sessions = append(sessions, s)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"sessions": sessions,
		"count":    len(sessions),
	})
}

// PUT /v1/insights/study-plan/:id - Update study session
func updateStudySession(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	sessionID := r.URL.Query().Get("id")

	if userID == "" || sessionID == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	var req struct {
		Status string `json:"status"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	var completedAt *time.Time
	if req.Status == "completed" {
		now := time.Now().UTC()
		completedAt = &now
	}

	_, err := db.Exec(
		"UPDATE study_sessions SET status = $1, completed_at = $2 WHERE id = $3 AND user_id = $4",
		req.Status, completedAt, sessionID, userID,
	)

	if err != nil {
		log.Printf("Error updating session: %v\n", err)
		http.Error(w, "Failed to update session", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "updated",
	})
}

// ============================================================================
// LEARNING STYLE ENDPOINTS
// ============================================================================

// GET /v1/insights/learning-style - Get user's learning style
func getLearningStyle(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var style LearningStyle
	err := db.QueryRow(`
		SELECT user_id, visual_percent, auditory_percent, kinesthetic_percent, reading_percent, dominant_style, updated_at
		FROM learning_styles
		WHERE user_id = $1
	`, userID).Scan(
		&style.UserID, &style.Visual, &style.Auditory, &style.Kinesthetic,
		&style.Reading, &style.DominantStyle, &style.UpdatedAt,
	)

	if err == sql.ErrNoRows {
		// Return defaults if not analyzed yet
		style = LearningStyle{
			UserID:        userID,
			Visual:        25,
			Auditory:      25,
			Kinesthetic:   25,
			Reading:       25,
			DominantStyle: "balanced",
			UpdatedAt:     time.Now().UTC(),
		}

		// Insert default
		_, _ = db.Exec(`
			INSERT INTO learning_styles (user_id, visual_percent, auditory_percent, kinesthetic_percent, reading_percent, dominant_style, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7)
		`,
			userID, 25, 25, 25, 25, "balanced", time.Now().UTC(),
		)
	} else if err != nil {
		log.Printf("Error getting learning style: %v\n", err)
		http.Error(w, "Failed to get learning style", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(style)
}

// PUT /v1/insights/learning-style - Update learning style (internal analysis)
func updateLearningStyle(w http.ResponseWriter, r *http.Request) {
	var req LearningStyle
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Determine dominant style
	styles := map[string]float64{
		"visual":      req.Visual,
		"auditory":    req.Auditory,
		"kinesthetic": req.Kinesthetic,
		"reading":     req.Reading,
	}

	var dominant string
	var max float64
	for style, percent := range styles {
		if percent > max {
			max = percent
			dominant = style
		}
	}

	now := time.Now().UTC()

	_, err := db.Exec(`
		INSERT INTO learning_styles (user_id, visual_percent, auditory_percent, kinesthetic_percent, reading_percent, dominant_style, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		ON CONFLICT (user_id) DO UPDATE SET
		visual_percent = $2, auditory_percent = $3, kinesthetic_percent = $4,
		reading_percent = $5, dominant_style = $6, updated_at = $7
	`,
		req.UserID, req.Visual, req.Auditory, req.Kinesthetic,
		req.Reading, dominant, now,
	)

	if err != nil {
		log.Printf("Error updating learning style: %v\n", err)
		http.Error(w, "Failed to update style", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "updated",
	})
}

// ============================================================================
// PERFORMANCE METRICS ENDPOINTS
// ============================================================================

// GET /v1/insights/performance - Get skill performance metrics
func getPerformanceMetrics(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT s.name, lp.mastery_percentage * 100,
	       SUM(CASE WHEN ea.was_correct THEN 1 ELSE 0 END)::float / NULLIF(COUNT(ea), 0) * 100,
	       COUNT(ea) as attempts,
	       COUNT(CASE WHEN ea.was_correct THEN 1 END) as correct,
	       CASE
	         WHEN lp.mastery_percentage > (SELECT AVG(mastery_percentage) FROM learner_progress) THEN 'up'
	         WHEN lp.mastery_percentage < (SELECT AVG(mastery_percentage) FROM learner_progress) * 0.95 THEN 'down'
	         ELSE 'stable'
	       END as trend
	FROM learner_progress lp
	JOIN skills s ON lp.skill_id = s.id
	LEFT JOIN exercise_attempts ea ON lp.user_id = ea.user_id AND lp.skill_id = s.id
	WHERE lp.user_id = $1
	GROUP BY s.id, s.name, lp.mastery_percentage
	ORDER BY lp.mastery_percentage DESC
	`

	rows, err := db.Query(query, userID)
	if err != nil {
		log.Printf("Error getting metrics: %v\n", err)
		http.Error(w, "Failed to get metrics", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var metrics []PerformanceMetric
	for rows.Next() {
		var m PerformanceMetric
		var attempts, correct int

		err := rows.Scan(
			&m.SkillName, &m.MasteryPercent, &m.AccuracyPercent,
			&attempts, &correct, &m.Trend,
		)
		if err != nil {
			log.Printf("Error scanning metric: %v\n", err)
			continue
		}

		m.ExercisesAttempted = attempts
		m.ExercisesCorrect = correct

		// Predict mastery based on current accuracy and progress
		if m.AccuracyPercent > m.MasteryPercent {
			m.PredictedMastery = math.Min(m.MasteryPercent+0.15, 1.0) * 100
		} else {
			m.PredictedMastery = m.MasteryPercent
		}

		metrics = append(metrics, m)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"metrics": metrics,
		"count":   len(metrics),
	})
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

	// Analytics endpoints
	r.HandleFunc("/v1/insights/analytics", getLearningAnalytics).Methods("GET")
	r.HandleFunc("/v1/insights/recommendations", getRecommendations).Methods("GET")
	r.HandleFunc("/v1/insights/recommendations/generate", generateRecommendations).Methods("POST")

	// Study planning endpoints
	r.HandleFunc("/v1/insights/study-plan", createStudySession).Methods("POST")
	r.HandleFunc("/v1/insights/study-plan", getStudyPlan).Methods("GET")
	r.HandleFunc("/v1/insights/study-plan/update", updateStudySession).Methods("PUT")

	// Learning style endpoints
	r.HandleFunc("/v1/insights/learning-style", getLearningStyle).Methods("GET")
	r.HandleFunc("/v1/insights/learning-style", updateLearningStyle).Methods("PUT")

	// Performance endpoints
	r.HandleFunc("/v1/insights/performance", getPerformanceMetrics).Methods("GET")

	// CORS
	c := cors.Default()
	handler := c.Handler(r)

	// Start server
	port := os.Getenv("INSIGHTS_SERVICE_PORT")
	if port == "" {
		port = "8009"
	}

	log.Printf("Learning Insights Service listening on port %s\n", port)
	log.Fatal(http.ListenAndServe(":"+port, handler))
}
