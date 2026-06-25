// PATHFINDER Parent Service
// Parent/Guardian account linking, progress monitoring, notifications
// Port: 8006

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

type ParentStudentLink struct {
	ID                string `json:"id"`
	ParentID          string `json:"parent_id"`
	StudentID         string `json:"student_id"`
	StudentName       string `json:"student_name"`
	StudentEmail      string `json:"student_email"`
	Relationship      string `json:"relationship"`
	Verified          bool   `json:"verified"`
	VerificationCode  string `json:"verification_code,omitempty"`
	VerificationSentAt string `json:"verification_sent_at,omitempty"`
	VerifiedAt        string `json:"verified_at,omitempty"`
	CreatedAt         string `json:"created_at"`
}

type StudentProgress struct {
	StudentID        string  `json:"student_id"`
	StudentName      string  `json:"student_name"`
	MasteryPercent   float64 `json:"mastery_percent"`
	SkillsMastered   int     `json:"skills_mastered"`
	TotalSkills      int     `json:"total_skills"`
	CurrentSkill     string  `json:"current_skill"`
	LastActivity     string  `json:"last_activity"`
	Status           string  `json:"status"`
	ExercisesToday   int     `json:"exercises_today"`
	AccuracyToday    float64 `json:"accuracy_today"`
	StreakDays       int     `json:"streak_days"`
}

type NotificationPreference struct {
	ID                    string `json:"id"`
	UserID                string `json:"user_id"`
	NotifyMastery         bool   `json:"notify_mastery"`
	NotifyAlerts          bool   `json:"notify_alerts"`
	NotifyDailySummary    bool   `json:"notify_daily_summary"`
	NotifyWeeklyReport    bool   `json:"notify_weekly_report"`
	NotifyAchievements    bool   `json:"notify_achievements"`
	EmailFrequency        string `json:"email_frequency"` // immediate, daily, weekly, never
	QuietHoursEnabled     bool   `json:"quiet_hours_enabled"`
	QuietHoursStart       string `json:"quiet_hours_start"`
	QuietHoursEnd         string `json:"quiet_hours_end"`
	Timezone              string `json:"timezone"`
	UpdatedAt             string `json:"updated_at"`
}

type Notification struct {
	ID        string `json:"id"`
	UserID    string `json:"user_id"`
	Type      string `json:"type"` // mastery, alert, summary, achievement
	Channel   string `json:"channel"` // email, push, sms
	Subject   string `json:"subject"`
	Message   string `json:"message"`
	SentAt    string `json:"sent_at"`
	OpenedAt  string `json:"opened_at,omitempty"`
	ClickedAt string `json:"clicked_at,omitempty"`
}

// ============================================================================
// PARENT LINKING ENDPOINTS
// ============================================================================

// POST /v1/parents/link-child - Link student to parent account
func linkChild(w http.ResponseWriter, r *http.Request) {
	var req struct {
		StudentEmail string `json:"student_email"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	parentID := r.Header.Get("X-User-ID")
	if parentID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Find student by email
	var studentID string
	var studentName string
	err := db.QueryRow(
		"SELECT id, CONCAT(first_name, ' ', last_name) FROM users WHERE email = $1",
		req.StudentEmail,
	).Scan(&studentID, &studentName)

	if err == sql.ErrNoRows {
		http.Error(w, "Student not found", http.StatusNotFound)
		return
	}
	if err != nil {
		log.Printf("Error finding student: %v\n", err)
		http.Error(w, "Failed to link student", http.StatusInternalServerError)
		return
	}

	// Create verification code
	verificationCode := uuid.New().String()[:8]

	// Insert link (unverified)
	linkID := uuid.New().String()
	now := time.Now().UTC().Format(time.RFC3339)

	_, err = db.Exec(`
	INSERT INTO parent_student_links (
		id, parent_id, student_id, relationship, verified,
		verification_code, verification_sent_at, created_at
	) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
	ON CONFLICT (parent_id, student_id) DO UPDATE SET
	verification_code = $6, verification_sent_at = $7
	`,
		linkID, parentID, studentID, "parent", false,
		verificationCode, now,
	)

	if err != nil {
		log.Printf("Error creating link: %v\n", err)
		http.Error(w, "Failed to link student", http.StatusInternalServerError)
		return
	}

	// TODO: Send verification code via email
	// SendVerificationEmail(req.StudentEmail, verificationCode)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"status": "verification code sent",
		"message": "Please ask your child to verify the code",
	})
}

// GET /v1/parents/children - Get linked children
func getLinkedChildren(w http.ResponseWriter, r *http.Request) {
	parentID := r.Header.Get("X-User-ID")
	if parentID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT psl.id, psl.parent_id, psl.student_id, u.first_name || ' ' || u.last_name,
	       u.email, psl.relationship, psl.verified, psl.created_at
	FROM parent_student_links psl
	JOIN users u ON psl.student_id = u.id
	WHERE psl.parent_id = $1 AND psl.verified = true
	ORDER BY u.first_name
	`

	rows, err := db.Query(query, parentID)
	if err != nil {
		log.Printf("Error getting children: %v\n", err)
		http.Error(w, "Failed to get children", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var children []ParentStudentLink
	for rows.Next() {
		var child ParentStudentLink
		err := rows.Scan(
			&child.ID, &child.ParentID, &child.StudentID, &child.StudentName,
			&child.StudentEmail, &child.Relationship, &child.Verified, &child.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning child: %v\n", err)
			continue
		}
		children = append(children, child)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"children": children,
		"count":    len(children),
	})
}

// GET /v1/parents/children/:id/progress - Get child's progress
func getChildProgress(w http.ResponseWriter, r *http.Request) {
	studentID := mux.Vars(r)["id"]
	parentID := r.Header.Get("X-User-ID")

	// Verify parent-child relationship
	var verified bool
	err := db.QueryRow(
		"SELECT verified FROM parent_student_links WHERE parent_id = $1 AND student_id = $2",
		parentID, studentID,
	).Scan(&verified)

	if err != nil || !verified {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get student progress
	query := `
	SELECT u.id, CONCAT(u.first_name, ' ', u.last_name),
	       COALESCE(lp.mastery_percentage, 0),
	       COALESCE(lp.mastered_skills, 0),
	       COALESCE(lp.total_skills, 0),
	       COALESCE(lp.current_skill, ''),
	       COALESCE(MAX(ea.created_at), ''),
	       CASE WHEN lp.mastery_percentage >= 0.85 THEN 'excellent'
	            WHEN lp.mastery_percentage >= 0.5 THEN 'developing'
	            ELSE 'struggling' END,
	       COUNT(CASE WHEN ea.created_at > NOW() - INTERVAL '1 day' THEN 1 END),
	       COALESCE(SUM(CASE WHEN ea.was_correct THEN 1 ELSE 0 END)::float / NULLIF(COUNT(ea), 0), 0),
	       COALESCE(lp.current_streak, 0)
	FROM users u
	LEFT JOIN learner_progress lp ON u.id = lp.user_id
	LEFT JOIN exercise_attempts ea ON u.id = ea.user_id
	WHERE u.id = $1
	GROUP BY u.id, u.first_name, u.last_name, lp.mastery_percentage,
	         lp.mastered_skills, lp.total_skills, lp.current_skill, lp.current_streak
	`

	var progress StudentProgress
	err = db.QueryRow(query, studentID).Scan(
		&progress.StudentID, &progress.StudentName,
		&progress.MasteryPercent, &progress.SkillsMastered,
		&progress.TotalSkills, &progress.CurrentSkill,
		&progress.LastActivity, &progress.Status,
		&progress.ExercisesToday, &progress.AccuracyToday,
		&progress.StreakDays,
	)

	if err != nil {
		log.Printf("Error getting progress: %v\n", err)
		http.Error(w, "Failed to get progress", http.StatusInternalServerError)
		return
	}

	progress.MasteryPercent = progress.MasteryPercent * 100
	progress.AccuracyToday = progress.AccuracyToday * 100

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(progress)
}

// ============================================================================
// NOTIFICATION ENDPOINTS
// ============================================================================

// POST /v1/notifications/preferences - Set notification preferences
func setNotificationPreferences(w http.ResponseWriter, r *http.Request) {
	var req NotificationPreference
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	INSERT INTO notification_preferences (
		id, user_id, notify_mastery, notify_alerts, notify_daily_summary,
		notify_weekly_report, notify_achievements, email_frequency,
		quiet_hours_enabled, quiet_hours_start, quiet_hours_end,
		timezone, updated_at
	) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
	ON CONFLICT (user_id) DO UPDATE SET
	notify_mastery = $3, notify_alerts = $4, notify_daily_summary = $5,
	notify_weekly_report = $6, notify_achievements = $7,
	email_frequency = $8, quiet_hours_enabled = $9,
	quiet_hours_start = $10, quiet_hours_end = $11, timezone = $12,
	updated_at = $13
	`

	now := time.Now().UTC().Format(time.RFC3339)
	_, err := db.Exec(query,
		uuid.New().String(), userID,
		req.NotifyMastery, req.NotifyAlerts, req.NotifyDailySummary,
		req.NotifyWeeklyReport, req.NotifyAchievements, req.EmailFrequency,
		req.QuietHoursEnabled, req.QuietHoursStart, req.QuietHoursEnd,
		req.Timezone, now,
	)

	if err != nil {
		log.Printf("Error setting preferences: %v\n", err)
		http.Error(w, "Failed to set preferences", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "preferences updated",
	})
}

// GET /v1/notifications/preferences - Get notification preferences
func getNotificationPreferences(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var prefs NotificationPreference
	err := db.QueryRow(`
	SELECT id, user_id, notify_mastery, notify_alerts, notify_daily_summary,
	       notify_weekly_report, notify_achievements, email_frequency,
	       quiet_hours_enabled, quiet_hours_start, quiet_hours_end,
	       timezone, updated_at
	FROM notification_preferences
	WHERE user_id = $1
	`, userID).Scan(
		&prefs.ID, &prefs.UserID,
		&prefs.NotifyMastery, &prefs.NotifyAlerts, &prefs.NotifyDailySummary,
		&prefs.NotifyWeeklyReport, &prefs.NotifyAchievements, &prefs.EmailFrequency,
		&prefs.QuietHoursEnabled, &prefs.QuietHoursStart, &prefs.QuietHoursEnd,
		&prefs.Timezone, &prefs.UpdatedAt,
	)

	if err == sql.ErrNoRows {
		// Return defaults
		prefs = NotificationPreference{
			UserID:                userID,
			NotifyMastery:         true,
			NotifyAlerts:          true,
			NotifyDailySummary:    true,
			NotifyWeeklyReport:    false,
			NotifyAchievements:    true,
			EmailFrequency:        "daily",
			QuietHoursEnabled:     false,
			Timezone:              "UTC",
		}
	} else if err != nil {
		log.Printf("Error getting preferences: %v\n", err)
		http.Error(w, "Failed to get preferences", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(prefs)
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

	// Parent linking endpoints
	r.HandleFunc("/v1/parents/link-child", linkChild).Methods("POST")
	r.HandleFunc("/v1/parents/children", getLinkedChildren).Methods("GET")
	r.HandleFunc("/v1/parents/children/{id}/progress", getChildProgress).Methods("GET")

	// Notification endpoints
	r.HandleFunc("/v1/notifications/preferences", setNotificationPreferences).Methods("POST")
	r.HandleFunc("/v1/notifications/preferences", getNotificationPreferences).Methods("GET")

	// CORS
	c := cors.Default()
	handler := c.Handler(r)

	// Start server
	port := os.Getenv("PARENT_SERVICE_PORT")
	if port == "" {
		port = "8006"
	}

	log.Printf("Parent Service listening on port %s\n", port)
	log.Fatal(http.ListenAndServe(":"+port, handler))
}
