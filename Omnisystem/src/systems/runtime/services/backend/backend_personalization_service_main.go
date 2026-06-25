// PATHFINDER Personalization Service - Learning Algorithms & Scheduling
// Phase 1: Week 2 Implementation
// Responsible for: BKT, HLR, exercise attempts, skill state updates, scheduling

package main

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"math"
	"net/http"
	"os"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/segmentio/kafka-go"
	_ "github.com/lib/pq"
)

// ============================================================================
// MODELS
// ============================================================================

type LearnerSkillState struct {
	ID                string    `json:"id"`
	UserID            string    `json:"user_id"`
	SkillID           string    `json:"skill_id"`
	PKnow             float64   `json:"p_know"`
	PSlip             float64   `json:"p_slip"`
	PGuess            float64   `json:"p_guess"`
	PTransit          float64   `json:"p_transit"`
	HalflifeDay       float64   `json:"halflife_day"`
	LastReviewedAt    *time.Time `json:"last_reviewed_at"`
	NextReviewAt      *time.Time `json:"next_review_at"`
	IsMastered        bool      `json:"is_mastered"`
	MasteryThreshold  float64   `json:"mastery_threshold"`
	Strength          float64   `json:"strength"`
	AttemptCount      int       `json:"attempt_count"`
	CorrectCount      int       `json:"correct_count"`
	CreatedAt         time.Time `json:"created_at"`
}

type ExerciseAttemptRequest struct {
	ExerciseID        string `json:"exercise_id" binding:"required"`
	SkillID           string `json:"skill_id" binding:"required"`
	WasCorrect        bool   `json:"was_correct" binding:"required"`
	Response          *string `json:"response"`
	ResponseTimeSeconds *int   `json:"response_time_seconds"`
}

type ExerciseAttemptResponse struct {
	SkillState        LearnerSkillState `json:"skill_state"`
	NextReviewAt      *time.Time `json:"next_review_at"`
	IsMastered        bool      `json:"is_mastered"`
	Feedback          string    `json:"feedback"`
}

type LearnerSkillsResponse struct {
	Skills []SkillStateWithExtraInfo `json:"skills"`
	Count  int                       `json:"count"`
}

type SkillStateWithExtraInfo struct {
	ID                string    `json:"id"`
	Code              string    `json:"skill_code"`
	Name              string    `json:"skill_name"`
	PKnow             float64   `json:"p_know"`
	Strength          float64   `json:"strength"`
	IsMastered        bool      `json:"is_mastered"`
	NextReviewAt      *time.Time `json:"next_review_at"`
	AttemptCount      int       `json:"attempt_count"`
	CorrectCount      int       `json:"correct_count"`
	CorrectRate       float64   `json:"correct_rate"`
	DaysUntilReview   *int      `json:"days_until_review"`
}

type NextSkillsResponse struct {
	Skills   []SkillToReview `json:"skills"`
	Count    int             `json:"count"`
	Priority string          `json:"priority"` // "overdue", "due_soon", "maintenance"
}

type SkillToReview struct {
	ID              string    `json:"id"`
	Code            string    `json:"skill_code"`
	Name            string    `json:"skill_name"`
	PKnow           float64   `json:"p_know"`
	NextReviewAt    *time.Time `json:"next_review_at"`
	DaysOverdue     *int      `json:"days_overdue"`
	IsMastered      bool      `json:"is_mastered"`
	ReviewPriority  int       `json:"review_priority"` // 1=urgent, 2=due, 3=soon
}

type ProgressMetrics struct {
	TotalSkills              int     `json:"total_skills"`
	MasteredSkills           int     `json:"mastered_skills"`
	DevelopingSkills         int     `json:"developing_skills"`
	MasteryPercentage        float64 `json:"mastery_percentage"`
	AverageStrength          float64 `json:"average_strength"`
	TotalExercisesCompleted  int     `json:"total_exercises_completed"`
	AverageAccuracy          float64 `json:"average_accuracy"`
	CurrentStreak            int     `json:"current_streak"`
	LongestStreak            int     `json:"longest_streak"`
}

type KafkaEvent struct {
	Type           string                 `json:"type"` // "exercise_attempt", "skill_mastered", "learner_progress"
	UserID         string                 `json:"user_id"`
	SkillID        string                 `json:"skill_id,omitempty"`
	ExerciseID     string                 `json:"exercise_id,omitempty"`
	WasCorrect     bool                   `json:"was_correct,omitempty"`
	Timestamp      time.Time              `json:"timestamp"`
	Metadata       map[string]interface{} `json:"metadata,omitempty"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
}

// ============================================================================
// BKT PARAMETERS
// ============================================================================

type BKTParams struct {
	PInit   float64 // Initial probability of knowing (0.3)
	PSlip   float64 // Probability of slip (0.1)
	PGuess  float64 // Probability of guess (0.25)
	PTransit float64 // Probability of learning (0.05)
}

// ============================================================================
// HLR PARAMETERS
// ============================================================================

type HLRParams struct {
	Decay     float64 // Decay rate (0.3)
	Threshold float64 // Retention threshold (0.9)
	MinDays   float64 // Minimum interval (1)
	MaxDays   float64 // Maximum interval (36000)
}

// ============================================================================
// SERVICE
// ============================================================================

type PersonalizationService struct {
	db        *sql.DB
	kafka     *kafka.Writer
	bktParams BKTParams
	hlrParams HLRParams
}

func NewPersonalizationService(db *sql.DB, kafkaBrokers []string) *PersonalizationService {
	// Initialize Kafka writer
	kafkaWriter := kafka.NewWriter(kafka.WriterConfig{
		Brokers:  kafkaBrokers,
		Topic:    "learning-events",
		Balancer: &kafka.LeastBytes{},
	})

	return &PersonalizationService{
		db:     db,
		kafka:  kafkaWriter,
		bktParams: BKTParams{
			PInit:    0.3,
			PSlip:    0.1,
			PGuess:   0.25,
			PTransit: 0.05,
		},
		hlrParams: HLRParams{
			Decay:     0.3,
			Threshold: 0.9,
			MinDays:   1,
			MaxDays:   36000,
		},
	}
}

// ============================================================================
// LEARNING SCIENCE ALGORITHMS
// ============================================================================

// UpdateBKTState updates probability of knowledge using Bayes' rule
func (s *PersonalizationService) updateBKTState(state *LearnerSkillState, isCorrect bool) {
	pKnow := state.PKnow
	pSlip := state.PSlip
	pGuess := state.PGuess

	// P(correct) = P(know) * (1 - P(slip)) + P(not know) * P(guess)
	pCorrect := pKnow*(1-pSlip) + (1-pKnow)*pGuess

	// Bayesian update
	if isCorrect {
		// P(know | correct) = P(correct | know) * P(know) / P(correct)
		state.PKnow = (pKnow * (1 - pSlip)) / pCorrect
	} else {
		// P(know | incorrect) = P(incorrect | know) * P(know) / P(incorrect)
		pIncorrect := 1 - pCorrect
		state.PKnow = (pKnow * pSlip) / pIncorrect
	}

	// Learning transition
	if state.PKnow < 0.99 { // Prevent overflow at 1.0
		state.PKnow = state.PKnow + (1-state.PKnow)*state.PTransit
	}

	// Strength is normalized P(Know)
	state.Strength = state.PKnow
}

// CalculateNextReviewInterval calculates optimal next review using Half-Life Regression
func (s *PersonalizationService) calculateNextReviewInterval(
	state *LearnerSkillState,
	lastAttemptCorrect bool,
) time.Duration {
	// Scale factor: wrong answer = review sooner
	factor := 1.0
	if !lastAttemptCorrect {
		factor = 0.5
	}

	// Stability increases with strength
	stability := 1.0 + state.Strength*5.0

	// Decay rate decreases with practice (more practice = slower decay)
	decay := s.hlrParams.Decay * (1.0 - (float64(state.AttemptCount) / 100.0))
	if decay < 0.1 {
		decay = 0.1
	}

	// Get current halflife (default 21 days)
	halfLife := state.HalflifeDay * 24 * 60 * 60 // Convert to seconds
	if halfLife == 0 {
		halfLife = 21 * 24 * 60 * 60 // Default 21 days
	}

	// Ideal retention interval: retention = 2^(-t/halflife)
	// Solving for t: t = -halflife * log2(desired_retention)
	desiredRetention := s.hlrParams.Threshold
	nextIntervalSeconds := -halfLife * math.Log(desiredRetention) / math.Log(2.0)

	// Apply factors
	nextIntervalSeconds *= factor

	// Clamp to bounds
	minSeconds := s.hlrParams.MinDays * 24 * 60 * 60
	maxSeconds := s.hlrParams.MaxDays * 24 * 60 * 60

	if nextIntervalSeconds < minSeconds {
		nextIntervalSeconds = minSeconds
	}
	if nextIntervalSeconds > maxSeconds {
		nextIntervalSeconds = maxSeconds
	}

	// Update state
	state.HalflifeDay = nextIntervalSeconds / (24 * 60 * 60)
	now := time.Now()
	state.LastReviewedAt = &now
	nextReview := now.Add(time.Duration(nextIntervalSeconds) * time.Second)
	state.NextReviewAt = &nextReview

	return time.Duration(nextIntervalSeconds) * time.Second
}

// ============================================================================
// EXERCISE ATTEMPT HANDLER
// ============================================================================

// RecordExerciseAttempt records an attempt and updates learner state
func (s *PersonalizationService) RecordExerciseAttempt(c *gin.Context) {
	userID := c.Param("user_id")

	var req ExerciseAttemptRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_INPUT",
			Message: err.Error(),
		})
		return
	}

	// Start transaction
	tx, err := s.db.BeginTx(c.Request.Context(), nil)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to start transaction",
		})
		return
	}
	defer tx.Rollback()

	// Insert exercise attempt
	attemptID := newUUID().String()
	now := time.Now()

	insertAttemptQuery := `
		INSERT INTO exercise_attempts
		(id, user_id, exercise_id, skill_id, was_correct, response, response_time_seconds, attempt_number, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8)
	`
	_, err = tx.ExecContext(c.Request.Context(), insertAttemptQuery,
		attemptID, userID, req.ExerciseID, req.SkillID,
		req.WasCorrect, req.Response, req.ResponseTimeSeconds, now,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to record attempt",
		})
		return
	}

	// Fetch or create skill state
	var state LearnerSkillState
	getStateQuery := `
		SELECT id, user_id, skill_id, p_know, p_slip, p_guess, p_transit,
		       halflife_days, last_reviewed_at, next_review_at, is_mastered,
		       mastery_threshold, strength, attempt_count, correct_count,
		       created_at, updated_at
		FROM learner_skill_states
		WHERE user_id = $1 AND skill_id = $2
	`

	var createdAt, updatedAt time.Time
	err = tx.QueryRowContext(c.Request.Context(), getStateQuery, userID, req.SkillID).Scan(
		&state.ID, &state.UserID, &state.SkillID, &state.PKnow, &state.PSlip, &state.PGuess, &state.PTransit,
		&state.HalflifeDay, &state.LastReviewedAt, &state.NextReviewAt, &state.IsMastered,
		&state.MasteryThreshold, &state.Strength, &state.AttemptCount, &state.CorrectCount,
		&createdAt, &updatedAt,
	)

	if err == sql.ErrNoRows {
		// Create new skill state
		state.ID = newUUID().String()
		state.UserID = userID
		state.SkillID = req.SkillID
		state.PKnow = 0.3 // Initial probability
		state.PSlip = 0.1
		state.PGuess = 0.25
		state.PTransit = 0.05
		state.MasteryThreshold = 0.85
		state.Strength = 0.3
		state.CreatedAt = now

		insertStateQuery := `
			INSERT INTO learner_skill_states
			(id, user_id, skill_id, p_know, p_slip, p_guess, p_transit,
			 halflife_days, mastery_threshold, strength, attempt_count, correct_count, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
		`
		_, err = tx.ExecContext(c.Request.Context(), insertStateQuery,
			state.ID, state.UserID, state.SkillID, state.PKnow, state.PSlip, state.PGuess, state.PTransit,
			state.HalflifeDay, state.MasteryThreshold, state.Strength, 0, 0, now, now,
		)
		if err != nil {
			c.JSON(http.StatusInternalServerError, ErrorResponse{
				Error:   "DATABASE_ERROR",
				Message: "Failed to create skill state",
			})
			return
		}
	} else if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch skill state",
		})
		return
	}

	// Update BKT state
	s.updateBKTState(&state, req.WasCorrect)

	// Update counts
	state.AttemptCount++
	if req.WasCorrect {
		state.CorrectCount++
	}

	// Check mastery
	wasMastered := state.IsMastered
	if state.PKnow >= state.MasteryThreshold && !state.IsMastered {
		state.IsMastered = true
	}

	// Calculate next review interval
	s.calculateNextReviewInterval(&state, req.WasCorrect)

	// Update skill state
	updateStateQuery := `
		UPDATE learner_skill_states
		SET p_know = $1, strength = $2, halflife_days = $3, last_reviewed_at = $4,
		    next_review_at = $5, is_mastered = $6, attempt_count = $7,
		    correct_count = $8, updated_at = $9
		WHERE id = $10
	`
	_, err = tx.ExecContext(c.Request.Context(), updateStateQuery,
		state.PKnow, state.Strength, state.HalflifeDay, state.LastReviewedAt,
		state.NextReviewAt, state.IsMastered, state.AttemptCount, state.CorrectCount,
		now, state.ID,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to update skill state",
		})
		return
	}

	// Record in review history
	reviewQuery := `
		INSERT INTO review_history
		(id, user_id, skill_id, review_number, interval_days, ease_factor, was_success, next_interval_days, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
	`

	var intervalDays *int
	var nextIntervalDays *int

	if state.LastReviewedAt != nil {
		days := int(time.Since(*state.LastReviewedAt).Hours() / 24)
		intervalDays = &days
	}

	if state.NextReviewAt != nil {
		days := int(time.Until(*state.NextReviewAt).Hours() / 24)
		nextIntervalDays = &days
	}

	_, _ = tx.ExecContext(c.Request.Context(), reviewQuery,
		newUUID().String(), userID, req.SkillID, state.AttemptCount,
		intervalDays, 2.5, req.WasCorrect, nextIntervalDays, now,
	)

	// Commit transaction
	if err = tx.Commit(); err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to commit transaction",
		})
		return
	}

	// Publish Kafka event
	go func() {
		event := KafkaEvent{
			Type:       "exercise_attempt",
			UserID:     userID,
			SkillID:    req.SkillID,
			ExerciseID: req.ExerciseID,
			WasCorrect: req.WasCorrect,
			Timestamp:  now,
			Metadata: map[string]interface{}{
				"p_know":     state.PKnow,
				"is_mastered": state.IsMastered,
				"attempt_number": state.AttemptCount,
			},
		}

		eventJSON, _ := json.Marshal(event)
		s.kafka.WriteMessages(context.Background(), kafka.Message{
			Key:   []byte(userID),
			Value: eventJSON,
		})
	}()

	// Generate feedback
	feedback := ""
	if req.WasCorrect {
		correctRate := float64(state.CorrectCount) / float64(state.AttemptCount)
		if correctRate > 0.8 {
			feedback = "Excellent work! You're mastering this skill."
		} else if correctRate > 0.6 {
			feedback = "Good progress! Keep practicing."
		} else {
			feedback = "You're learning. This skill will be reviewed soon."
		}
	} else {
		feedback = "Incorrect. This will be reviewed in the optimal interval to help you remember."
	}

	response := ExerciseAttemptResponse{
		SkillState:   state,
		NextReviewAt: state.NextReviewAt,
		IsMastered:   state.IsMastered,
		Feedback:     feedback,
	}

	c.JSON(http.StatusOK, response)
}

// ============================================================================
// SKILL STATE HANDLERS
// ============================================================================

// GetLearnerSkills returns all skills and their states
func (s *PersonalizationService) GetLearnerSkills(c *gin.Context) {
	userID := c.Param("user_id")

	query := `
		SELECT lss.id, lss.user_id, lss.skill_id, lss.p_know, lss.strength,
		       lss.is_mastered, lss.next_review_at, lss.attempt_count, lss.correct_count,
		       sk.code, sk.name
		FROM learner_skill_states lss
		INNER JOIN skills sk ON lss.skill_id = sk.id
		WHERE lss.user_id = $1
		ORDER BY lss.next_review_at ASC NULLS LAST
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query, userID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch skills",
		})
		return
	}
	defer rows.Close()

	skills := []SkillStateWithExtraInfo{}
	for rows.Next() {
		var skill SkillStateWithExtraInfo
		var nextReview *time.Time

		err := rows.Scan(
			&skill.ID, &skill.ID, &skill.ID, &skill.PKnow, &skill.Strength,
			&skill.IsMastered, &nextReview, &skill.AttemptCount, &skill.CorrectCount,
			&skill.Code, &skill.Name,
		)
		if err != nil {
			continue
		}

		skill.NextReviewAt = nextReview

		// Calculate correct rate
		if skill.AttemptCount > 0 {
			skill.CorrectRate = float64(skill.CorrectCount) / float64(skill.AttemptCount)
		}

		// Calculate days until review
		if nextReview != nil {
			daysUntil := int(time.Until(*nextReview).Hours() / 24)
			skill.DaysUntilReview = &daysUntil
		}

		skills = append(skills, skill)
	}

	c.JSON(http.StatusOK, LearnerSkillsResponse{
		Skills: skills,
		Count:  len(skills),
	})
}

// GetNextSkillsToReview returns skills ready for review (spaced repetition)
func (s *PersonalizationService) GetNextSkillsToReview(c *gin.Context) {
	userID := c.Param("user_id")
	limitStr := c.DefaultQuery("limit", "10")

	var limit int
	fmt.Sscanf(limitStr, "%d", &limit)
	if limit > 100 {
		limit = 100
	}

	query := `
		SELECT lss.id, lss.skill_id, lss.p_know, lss.next_review_at, lss.is_mastered,
		       sk.code, sk.name,
		       CAST(EXTRACT(DAY FROM NOW() - lss.next_review_at) AS INTEGER) as days_overdue
		FROM learner_skill_states lss
		INNER JOIN skills sk ON lss.skill_id = sk.id
		WHERE lss.user_id = $1 AND lss.next_review_at IS NOT NULL
		ORDER BY
		  CASE
		    WHEN lss.next_review_at < NOW() THEN 1
		    WHEN lss.next_review_at < NOW() + INTERVAL '7 days' THEN 2
		    ELSE 3
		  END,
		  lss.next_review_at ASC
		LIMIT $2
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query, userID, limit)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch skills to review",
		})
		return
	}
	defer rows.Close()

	skills := []SkillToReview{}
	for rows.Next() {
		var skill SkillToReview
		var nextReview *time.Time
		var daysOverdue *int

		err := rows.Scan(
			&skill.ID, &skill.ID, &skill.PKnow, &nextReview, &skill.IsMastered,
			&skill.Code, &skill.Name, &daysOverdue,
		)
		if err != nil {
			continue
		}

		skill.NextReviewAt = nextReview
		skill.DaysOverdue = daysOverdue

		// Set priority
		if daysOverdue != nil && *daysOverdue > 0 {
			skill.ReviewPriority = 1 // Urgent
		} else if nextReview != nil && time.Until(*nextReview) < 24*time.Hour {
			skill.ReviewPriority = 2 // Due soon
		} else {
			skill.ReviewPriority = 3 // Upcoming
		}

		skills = append(skills, skill)
	}

	priority := "normal"
	if len(skills) > 0 && skills[0].ReviewPriority == 1 {
		priority = "overdue"
	}

	c.JSON(http.StatusOK, NextSkillsResponse{
		Skills:   skills,
		Count:    len(skills),
		Priority: priority,
	})
}

// GetProgress returns overall learner progress metrics
func (s *PersonalizationService) GetProgress(c *gin.Context) {
	userID := c.Param("user_id")

	query := `
		SELECT
		  COUNT(*) as total_skills,
		  SUM(CASE WHEN is_mastered THEN 1 ELSE 0 END) as mastered_skills,
		  AVG(strength) as avg_strength,
		  SUM(attempt_count) as total_attempts,
		  SUM(correct_count) as total_correct
		FROM learner_skill_states
		WHERE user_id = $1
	`

	var totalSkills, masteredSkills sql.NullInt64
	var avgStrength, totalAttempts, totalCorrect sql.NullFloat64

	err := s.db.QueryRowContext(c.Request.Context(), query, userID).Scan(
		&totalSkills, &masteredSkills, &avgStrength, &totalAttempts, &totalCorrect,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to calculate progress",
		})
		return
	}

	metrics := ProgressMetrics{
		TotalSkills:             int(totalSkills.Int64),
		MasteredSkills:          int(masteredSkills.Int64),
		AverageStrength:         avgStrength.Float64,
		TotalExercisesCompleted: int(totalAttempts.Float64),
	}

	if metrics.TotalSkills > 0 {
		metrics.DevelopingSkills = metrics.TotalSkills - metrics.MasteredSkills
		metrics.MasteryPercentage = float64(metrics.MasteredSkills) / float64(metrics.TotalSkills) * 100
	}

	if totalAttempts.Float64 > 0 {
		metrics.AverageAccuracy = totalCorrect.Float64 / totalAttempts.Float64
	}

	// Calculate streak (simplified - would be more complex in production)
	metrics.CurrentStreak = 0
	metrics.LongestStreak = 0

	c.JSON(http.StatusOK, metrics)
}

// Health check
func (s *PersonalizationService) Health(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "healthy",
		"service": "personalization-service",
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

	kafkaBrokers := []string{"localhost:9092"}
	if brokers := os.Getenv("KAFKA_BROKERS"); brokers != "" {
		kafkaBrokers = []string{brokers}
	}

	port := os.Getenv("PERSONALIZATION_SERVICE_PORT")
	if port == "" {
		port = "8003"
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
	service := NewPersonalizationService(db, kafkaBrokers)

	// Setup router
	router := gin.Default()

	// Health check
	router.GET("/health", service.Health)

	// Learner endpoints
	router.GET("/v1/learners/:user_id/skills", service.GetLearnerSkills)
	router.GET("/v1/learners/:user_id/next-skills", service.GetNextSkillsToReview)
	router.POST("/v1/learners/:user_id/exercises/:exercise_id/attempt", service.RecordExerciseAttempt)
	router.GET("/v1/learners/:user_id/progress", service.GetProgress)

	// Start server
	log.Printf("Starting Personalization Service on port %s", port)
	if err := router.Run(":" + port); err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

type UUID [16]byte

func newUUID() UUID {
	// Placeholder - use github.com/google/uuid in production
	var u UUID
	return u
}

func (u UUID) String() string {
	return fmt.Sprintf("%x-%x-%x-%x-%x",
		u[0:4], u[4:6], u[6:8], u[8:10], u[10:])
}
