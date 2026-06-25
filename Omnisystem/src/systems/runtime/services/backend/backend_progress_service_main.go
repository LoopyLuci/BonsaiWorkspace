// PATHFINDER Progress Service - Analytics & Learning Curves
// Phase 1: Week 3 Implementation
// Responsible for: Analytics, metrics, learning curves, achievements, cohort stats

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

	"github.com/gin-gonic/gin"
	_ "github.com/lib/pq"
)

// ============================================================================
// MODELS
// ============================================================================

type DailyMetrics struct {
	Date                time.Time `json:"date"`
	ExercisesAttempted  int       `json:"exercises_attempted"`
	ExercisesCorrect    int       `json:"exercises_correct"`
	CorrectRate         float64   `json:"correct_rate"`
	TimeSpentSeconds    int       `json:"time_spent_seconds"`
	SkillsReviewed      int       `json:"skills_reviewed"`
	NewSkillsMastered   int       `json:"new_skills_mastered"`
	IsStreakDay         bool      `json:"is_streak_day"`
	XPEarned            int       `json:"xp_earned"`
}

type MonthlyMetrics struct {
	Month              string  `json:"month"`
	TotalExercises     int     `json:"total_exercises"`
	CorrectRate        float64 `json:"correct_rate"`
	TotalTimeSpent     int     `json:"total_time_spent_seconds"`
	AverageSessionTime int     `json:"average_session_time_seconds"`
	SkillsReviewed     int     `json:"skills_reviewed"`
	SkillsMastered     int     `json:"skills_mastered"`
	ConsistencyDays    int     `json:"consistency_days"`
}

type LearningCurvePoint struct {
	Date            time.Time `json:"date"`
	SkillID         string    `json:"skill_id"`
	PKnow           float64   `json:"p_know"`
	Strength        float64   `json:"strength"`
	CorrectRate     float64   `json:"correct_rate"`
	AttemptCount    int       `json:"attempt_count"`
}

type ProgressVisualization struct {
	SkillID         string                 `json:"skill_id"`
	SkillName       string                 `json:"skill_name"`
	LearningCurve   []LearningCurvePoint   `json:"learning_curve"`
	StartDate       time.Time              `json:"start_date"`
	LastReviewDate  time.Time              `json:"last_review_date"`
	MasteryProgress float64                `json:"mastery_progress"`
	TrendDirection  string                 `json:"trend_direction"` // "improving", "stable", "declining"
}

type CohortMetrics struct {
	CohortID              string  `json:"cohort_id"`
	ClassroomName         string  `json:"classroom_name"`
	StudentCount          int     `json:"student_count"`
	AverageMasteryRate    float64 `json:"average_mastery_rate"`
	AverageAccuracy       float64 `json:"average_accuracy"`
	EngagementRate        float64 `json:"engagement_rate"`
	AverageExercisesPerDay int    `json:"average_exercises_per_day"`
	StrugglingSluggishRepeated string `json:"struggling_students"` // JSON list
	TopPerformers          string  `json:"top_performers"`        // JSON list
}

type LearnerComparison struct {
	StudentName     string  `json:"student_name"`
	SkillsMastered  int     `json:"skills_mastered"`
	AverageAccuracy float64 `json:"average_accuracy"`
	ConsecutiveDays int     `json:"consecutive_days"`
	ExercisesToday  int     `json:"exercises_today"`
	Status          string  `json:"status"` // "on_track", "needs_help", "excelling"
}

type ExerciseStatistics struct {
	ExerciseID         string  `json:"exercise_id"`
	ExerciseTitle      string  `json:"exercise_title"`
	SkillID            string  `json:"skill_id"`
	AttemptCount       int     `json:"attempt_count"`
	SuccessRate        float64 `json:"success_rate"`
	AverageTimeSeconds int     `json:"average_time_seconds"`
	DifficultyRating   float64 `json:"difficulty_rating"`
	DiscriminationIndex float64 `json:"discrimination_index"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
}

// ============================================================================
// SERVICE
// ============================================================================

type ProgressService struct {
	db *sql.DB
}

func NewProgressService(db *sql.DB) *ProgressService {
	return &ProgressService{db: db}
}

// ============================================================================
// DAILY METRICS
// ============================================================================

// GetDailyMetrics returns metrics for a specific day
func (s *ProgressService) GetDailyMetrics(c *gin.Context) {
	userID := c.Param("user_id")
	dateStr := c.Query("date")

	// Parse date or use today
	var date time.Time
	if dateStr != "" {
		var err error
		date, err = time.Parse("2006-01-02", dateStr)
		if err != nil {
			c.JSON(http.StatusBadRequest, ErrorResponse{
				Error:   "INVALID_DATE",
				Message: "Use format YYYY-MM-DD",
			})
			return
		}
	} else {
		date = time.Now().Truncate(24 * time.Hour)
	}

	nextDay := date.AddDate(0, 0, 1)

	// Calculate metrics from exercise_attempts
	query := `
		SELECT
		  COUNT(*) as exercises_attempted,
		  SUM(CASE WHEN was_correct THEN 1 ELSE 0 END) as exercises_correct,
		  COALESCE(SUM(response_time_seconds), 0) as total_time
		FROM exercise_attempts
		WHERE user_id = $1 AND created_at >= $2 AND created_at < $3
	`

	var attemptsCount, correctCount, totalTimeSeconds sql.NullInt64
	err := s.db.QueryRowContext(c.Request.Context(), query, userID, date, nextDay).Scan(
		&attemptsCount, &correctCount, &totalTimeSeconds,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to calculate metrics",
		})
		return
	}

	metrics := DailyMetrics{
		Date:             date,
		ExercisesAttempted: int(attemptsCount.Int64),
		ExercisesCorrect:  int(correctCount.Int64),
		TimeSpentSeconds: int(totalTimeSeconds.Int64),
	}

	if metrics.ExercisesAttempted > 0 {
		metrics.CorrectRate = float64(metrics.ExercisesCorrect) / float64(metrics.ExercisesAttempted)
	}

	// Calculate XP earned (simplified)
	metrics.XPEarned = metrics.ExercisesCorrect * 10

	// Check if streak day
	metrics.IsStreakDay = metrics.ExercisesAttempted > 0

	c.JSON(http.StatusOK, metrics)
}

// GetMonthlyMetrics returns aggregated monthly metrics
func (s *ProgressService) GetMonthlyMetrics(c *gin.Context) {
	userID := c.Param("user_id")
	monthStr := c.Query("month") // Format: "2026-06"

	if monthStr == "" {
		now := time.Now()
		monthStr = fmt.Sprintf("%d-%02d", now.Year(), now.Month())
	}

	// Parse month
	startDate, err := time.Parse("2006-01", monthStr)
	if err != nil {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_MONTH",
			Message: "Use format YYYY-MM",
		})
		return
	}

	endDate := startDate.AddDate(0, 1, 0)

	query := `
		SELECT
		  COUNT(*) as total_exercises,
		  SUM(CASE WHEN was_correct THEN 1 ELSE 0 END) as exercises_correct,
		  COALESCE(SUM(response_time_seconds), 0) as total_time,
		  COUNT(DISTINCT DATE(created_at)) as practice_days
		FROM exercise_attempts
		WHERE user_id = $1 AND created_at >= $2 AND created_at < $3
	`

	var totalEx, correctEx, totalTime, practiceDays sql.NullInt64
	err = s.db.QueryRowContext(c.Request.Context(), query, userID, startDate, endDate).Scan(
		&totalEx, &correctEx, &totalTime, &practiceDays,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to calculate metrics",
		})
		return
	}

	metrics := MonthlyMetrics{
		Month:            monthStr,
		TotalExercises:   int(totalEx.Int64),
		TotalTimeSpent:   int(totalTime.Int64),
		ConsistencyDays:  int(practiceDays.Int64),
	}

	if metrics.TotalExercises > 0 {
		metrics.CorrectRate = float64(correctEx.Int64) / float64(metrics.TotalExercises)
	}

	if practiceDays.Int64 > 0 {
		metrics.AverageSessionTime = int(totalTime.Int64) / int(practiceDays.Int64)
	}

	// Get skill stats for month
	skillQuery := `
		SELECT COUNT(DISTINCT skill_id) as skills_reviewed,
		       SUM(CASE WHEN new_mastered THEN 1 ELSE 0 END) as skills_mastered
		FROM (
		  SELECT DISTINCT ON (skill_id) skill_id,
		         (SELECT is_mastered FROM learner_skill_states
		          WHERE skill_id = ea.skill_id AND user_id = $1) as new_mastered
		  FROM exercise_attempts ea
		  WHERE user_id = $1 AND created_at >= $2 AND created_at < $3
		) subq
	`

	var skillsReviewed, skillsMastered sql.NullInt64
	_ = s.db.QueryRowContext(c.Request.Context(), skillQuery, userID, startDate, endDate).Scan(
		&skillsReviewed, &skillsMastered,
	)
	metrics.SkillsReviewed = int(skillsReviewed.Int64)
	metrics.SkillsMastered = int(skillsMastered.Int64)

	c.JSON(http.StatusOK, metrics)
}

// ============================================================================
// LEARNING CURVES
// ============================================================================

// GetLearningCurve returns the learning curve for a specific skill
func (s *ProgressService) GetLearningCurve(c *gin.Context) {
	userID := c.Param("user_id")
	skillID := c.Param("skill_id")

	query := `
		SELECT ea.created_at::DATE as date,
		       lss.p_know, lss.strength, lss.attempt_count, lss.correct_count
		FROM exercise_attempts ea
		INNER JOIN learner_skill_states lss ON ea.skill_id = lss.skill_id
		WHERE ea.user_id = $1 AND ea.skill_id = $2
		GROUP BY ea.created_at::DATE, lss.p_know, lss.strength, lss.attempt_count, lss.correct_count
		ORDER BY ea.created_at::DATE ASC
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query, userID, skillID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch learning curve",
		})
		return
	}
	defer rows.Close()

	points := []LearningCurvePoint{}
	var startDate, lastDate time.Time

	for rows.Next() {
		var date time.Time
		var pKnow, strength float64
		var attemptCount, correctCount int

		err := rows.Scan(&date, &pKnow, &strength, &attemptCount, &correctCount)
		if err != nil {
			continue
		}

		correctRate := 0.0
		if attemptCount > 0 {
			correctRate = float64(correctCount) / float64(attemptCount)
		}

		point := LearningCurvePoint{
			Date:         date,
			SkillID:      skillID,
			PKnow:        pKnow,
			Strength:     strength,
			CorrectRate:  correctRate,
			AttemptCount: attemptCount,
		}

		points = append(points, point)

		if startDate.IsZero() {
			startDate = date
		}
		lastDate = date
	}

	// Get skill name
	var skillName string
	_ = s.db.QueryRowContext(c.Request.Context(), "SELECT name FROM skills WHERE id = $1", skillID).Scan(&skillName)

	// Calculate trend
	trendDirection := "stable"
	if len(points) > 1 {
		firstStrength := points[0].Strength
		lastStrength := points[len(points)-1].Strength
		change := lastStrength - firstStrength

		if change > 0.1 {
			trendDirection = "improving"
		} else if change < -0.1 {
			trendDirection = "declining"
		}
	}

	// Calculate mastery progress
	masteryProgress := 0.0
	if len(points) > 0 {
		masteryProgress = math.Min(1.0, points[len(points)-1].Strength)
	}

	visualization := ProgressVisualization{
		SkillID:        skillID,
		SkillName:      skillName,
		LearningCurve:  points,
		StartDate:      startDate,
		LastReviewDate: lastDate,
		MasteryProgress: masteryProgress,
		TrendDirection: trendDirection,
	}

	c.JSON(http.StatusOK, visualization)
}

// ============================================================================
// COHORT ANALYTICS (for teachers)
// ============================================================================

// GetCohortMetrics returns classroom-level statistics
func (s *ProgressService) GetCohortMetrics(c *gin.Context) {
	classroomID := c.Param("classroom_id")

	// Get classroom name
	var classroomName string
	_ = s.db.QueryRowContext(c.Request.Context(),
		"SELECT name FROM classrooms WHERE id = $1", classroomID).Scan(&classroomName)

	query := `
		SELECT
		  COUNT(DISTINCT ce.student_id) as student_count,
		  AVG(lss.p_know) as avg_mastery,
		  (SELECT COUNT(*) FROM learner_skill_states WHERE is_mastered)::FLOAT /
		    NULLIF((SELECT COUNT(*) FROM learner_skill_states), 0) as total_mastery,
		  AVG(CASE WHEN ea.was_correct THEN 1 ELSE 0 END) as avg_accuracy
		FROM classroom_enrollments ce
		LEFT JOIN learner_skill_states lss ON ce.student_id = lss.user_id
		LEFT JOIN exercise_attempts ea ON ce.student_id = ea.user_id
		WHERE ce.classroom_id = $1
	`

	var studentCount sql.NullInt64
	var avgMastery, avgAccuracy sql.NullFloat64

	err := s.db.QueryRowContext(c.Request.Context(), query, classroomID).Scan(
		&studentCount, &avgMastery, &avgAccuracy,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to calculate cohort metrics",
		})
		return
	}

	// Get struggling students
	strugglingQuery := `
		SELECT u.first_name || ' ' || COALESCE(u.last_name, '') as name,
		       COUNT(DISTINCT lss.skill_id) as skills_mastered,
		       AVG(ea.was_correct::INT) as accuracy
		FROM classroom_enrollments ce
		JOIN users u ON ce.student_id = u.id
		LEFT JOIN learner_skill_states lss ON ce.student_id = lss.user_id AND lss.is_mastered
		LEFT JOIN exercise_attempts ea ON ce.student_id = ea.user_id
		WHERE ce.classroom_id = $1
		GROUP BY ce.student_id, u.first_name, u.last_name
		HAVING AVG(ea.was_correct::INT) < 0.6
		ORDER BY accuracy ASC
		LIMIT 5
	`

	rows, _ := s.db.QueryContext(c.Request.Context(), strugglingQuery, classroomID)
	defer rows.Close()

	var struggling []LearnerComparison
	for rows.Next() {
		var student LearnerComparison
		var accuracy sql.NullFloat64
		rows.Scan(&student.StudentName, &student.SkillsMastered, &accuracy)
		if accuracy.Valid {
			student.AverageAccuracy = accuracy.Float64
		}
		student.Status = "needs_help"
		struggling = append(struggling, student)
	}

	strugglJSON, _ := json.Marshal(struggling)

	metrics := CohortMetrics{
		CohortID:           classroomID,
		ClassroomName:      classroomName,
		StudentCount:       int(studentCount.Int64),
		AverageMasteryRate: avgMastery.Float64 * 100,
		AverageAccuracy:    avgAccuracy.Float64 * 100,
		EngagementRate:     float64(studentCount.Int64) * 100 / float64(studentCount.Int64+1), // Placeholder
		StrugglingSluggishRepeated: string(strugglJSON),
	}

	c.JSON(http.StatusOK, metrics)
}

// ============================================================================
// EXERCISE STATISTICS
// ============================================================================

// GetExerciseStats returns difficulty analysis for exercises
func (s *ProgressService) GetExerciseStats(c *gin.Context) {
	skillID := c.Param("skill_id")

	query := `
		SELECT e.id, e.title, e.skill_id,
		       COUNT(*) as attempt_count,
		       SUM(CASE WHEN was_correct THEN 1 ELSE 0 END)::FLOAT / COUNT(*) as success_rate,
		       AVG(response_time_seconds) as avg_time
		FROM exercises e
		LEFT JOIN exercise_attempts ea ON e.id = ea.exercise_id
		WHERE e.skill_id = $1
		GROUP BY e.id, e.title, e.skill_id
		ORDER BY attempt_count DESC
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query, skillID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch exercise stats",
		})
		return
	}
	defer rows.Close()

	stats := []ExerciseStatistics{}
	for rows.Next() {
		var stat ExerciseStatistics
		var avgTime sql.NullInt64

		err := rows.Scan(&stat.ExerciseID, &stat.ExerciseTitle, &stat.SkillID,
			&stat.AttemptCount, &stat.SuccessRate, &avgTime)
		if err != nil {
			continue
		}

		if avgTime.Valid {
			stat.AverageTimeSeconds = int(avgTime.Int64)
		}

		// Calculate discrimination index (simplified)
		// Higher = better at distinguishing learner ability
		stat.DiscriminationIndex = (stat.SuccessRate - 0.5) * 2
		if stat.DiscriminationIndex < 0 {
			stat.DiscriminationIndex = 0
		}

		stats = append(stats, stat)
	}

	c.JSON(http.StatusOK, gin.H{
		"exercises": stats,
		"count":     len(stats),
	})
}

// Health check
func (s *ProgressService) Health(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "healthy",
		"service": "progress-service",
		"time":    time.Now().Format(time.RFC3339),
	})
}

// ============================================================================
// MAIN APPLICATION
// ============================================================================

func main() {
	// Environment setup
	databaseURL := os.Getenv("DATABASE_URL")
	if databaseURL == "" {
		databaseURL = "postgres://pathfinder:pathfinder_dev_password@localhost:5432/pathfinder"
	}

	port := os.Getenv("PROGRESS_SERVICE_PORT")
	if port == "" {
		port = "8004"
	}

	// Connect to database
	db, err := sql.Open("postgres", databaseURL)
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}
	defer db.Close()

	// Verify connection
	if err = db.Ping(); err != nil {
		log.Fatalf("Failed to ping database: %v", err)
	}

	log.Println("✓ Database connected")

	// Create service
	service := NewProgressService(db)

	// Setup router
	router := gin.Default()

	// Health check
	router.GET("/health", service.Health)

	// Analytics endpoints
	router.GET("/v1/learners/:user_id/daily-metrics", service.GetDailyMetrics)
	router.GET("/v1/learners/:user_id/monthly-metrics", service.GetMonthlyMetrics)

	// Learning curves
	router.GET("/v1/learners/:user_id/skills/:skill_id/learning-curve", service.GetLearningCurve)

	// Cohort (classroom) analytics
	router.GET("/v1/analytics/cohort/:classroom_id/metrics", service.GetCohortMetrics)

	// Exercise statistics
	router.GET("/v1/analytics/skills/:skill_id/exercises", service.GetExerciseStats)

	// Start server
	log.Printf("Starting Progress Service on port %s", port)
	if err := router.Run(":" + port); err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}
