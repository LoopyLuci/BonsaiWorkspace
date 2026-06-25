// PATHFINDER Content Service - Skills, Exercises, Lessons
// Phase 1: Week 1-4 Implementation
// Responsible for: Skill ontology, exercises, lessons, curriculum

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
	_ "github.com/lib/pq"
)

// ============================================================================
// MODELS
// ============================================================================

type Skill struct {
	ID                   string    `json:"id"`
	Code                 string    `json:"code"`
	Name                 string    `json:"name"`
	Description          *string   `json:"description"`
	Level                string    `json:"level"` // A1, A2, B1, etc
	Language             string    `json:"language"`
	Category             string    `json:"category"`
	IconURL              *string   `json:"icon_url"`
	ColorHex             *string   `json:"color_hex"`
	EstimatedTimeMinutes int       `json:"estimated_time_minutes"`
	DifficultyLevel      float64   `json:"difficulty_level"`
	IsPublished          bool      `json:"is_published"`
	CreatedAt            time.Time `json:"created_at"`
	Prerequisites        []string  `json:"prerequisites,omitempty"`
}

type Exercise struct {
	ID                    string    `json:"id"`
	SkillID               string    `json:"skill_id"`
	Type                  string    `json:"type"` // multiple_choice, translation, listening, etc
	Title                 string    `json:"title"`
	Description           *string   `json:"description"`
	DifficultyDelta       float64   `json:"difficulty_delta"`
	Prompt                *string   `json:"prompt"`
	CorrectOption         *int      `json:"correct_option"`
	Options               []string  `json:"options"`
	Explanation           *string   `json:"explanation"`
	IsPublished           bool      `json:"is_published"`
	EstimatedTimeSeconds  int       `json:"estimated_time_seconds"`
	UsageCount            int       `json:"usage_count"`
	AverageSuccessRate    float64   `json:"average_success_rate"`
	CreatedAt             time.Time `json:"created_at"`
}

type Lesson struct {
	ID                   string    `json:"id"`
	SkillID              string    `json:"skill_id"`
	Sequence             int       `json:"sequence"`
	Title                string    `json:"title"`
	Description          *string   `json:"description"`
	LearningObjectives   []string  `json:"learning_objectives"`
	Exercises            []Exercise `json:"exercises,omitempty"`
	CreatedAt            time.Time `json:"created_at"`
}

type CurriculumPath struct {
	ID          string   `json:"id"`
	Code        string   `json:"code"`
	Name        string   `json:"name"`
	Description *string  `json:"description"`
	Language    *string  `json:"language"`
	Skills      []Skill  `json:"skills,omitempty"`
	IsPublished bool     `json:"is_published"`
	CreatedAt   time.Time `json:"created_at"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
}

// ============================================================================
// SERVICE
// ============================================================================

type ContentService struct {
	db *sql.DB
}

func NewContentService(db *sql.DB) *ContentService {
	return &ContentService{db: db}
}

// ============================================================================
// SKILL HANDLERS
// ============================================================================

// ListSkills returns all published skills with optional filtering
func (s *ContentService) ListSkills(c *gin.Context) {
	language := c.DefaultQuery("language", "")
	level := c.DefaultQuery("level", "")
	category := c.DefaultQuery("category", "")

	query := "SELECT id, code, name, description, level, language, category, icon_url, color_hex, estimated_time_minutes, difficulty_level, is_published, created_at FROM skills WHERE is_published = true"
	args := []interface{}{}
	argCount := 1

	if language != "" {
		query += fmt.Sprintf(" AND language = $%d", argCount)
		args = append(args, language)
		argCount++
	}
	if level != "" {
		query += fmt.Sprintf(" AND level = $%d", argCount)
		args = append(args, level)
		argCount++
	}
	if category != "" {
		query += fmt.Sprintf(" AND category = $%d", argCount)
		args = append(args, category)
		argCount++
	}

	query += " ORDER BY level ASC, created_at ASC"

	rows, err := s.db.QueryContext(c.Request.Context(), query, args...)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch skills",
		})
		return
	}
	defer rows.Close()

	skills := []Skill{}
	for rows.Next() {
		var skill Skill
		err := rows.Scan(
			&skill.ID, &skill.Code, &skill.Name, &skill.Description, &skill.Level,
			&skill.Language, &skill.Category, &skill.IconURL, &skill.ColorHex,
			&skill.EstimatedTimeMinutes, &skill.DifficultyLevel, &skill.IsPublished, &skill.CreatedAt,
		)
		if err != nil {
			continue
		}
		skills = append(skills, skill)
	}

	if len(skills) == 0 {
		skills = []Skill{}
	}

	c.JSON(http.StatusOK, gin.H{
		"skills": skills,
		"count":  len(skills),
	})
}

// GetSkill returns a single skill with prerequisites
func (s *ContentService) GetSkill(c *gin.Context) {
	skillID := c.Param("skill_id")

	var skill Skill
	query := "SELECT id, code, name, description, level, language, category, icon_url, color_hex, estimated_time_minutes, difficulty_level, is_published, created_at FROM skills WHERE id = $1"

	err := s.db.QueryRowContext(c.Request.Context(), query, skillID).Scan(
		&skill.ID, &skill.Code, &skill.Name, &skill.Description, &skill.Level,
		&skill.Language, &skill.Category, &skill.IconURL, &skill.ColorHex,
		&skill.EstimatedTimeMinutes, &skill.DifficultyLevel, &skill.IsPublished, &skill.CreatedAt,
	)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusNotFound, ErrorResponse{
			Error:   "NOT_FOUND",
			Message: "Skill not found",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch skill",
		})
		return
	}

	// Fetch prerequisites
	preqQuery := `
		SELECT s.code FROM skills s
		INNER JOIN skill_prerequisites sp ON s.id = sp.prerequisite_skill_id
		WHERE sp.skill_id = $1
	`
	preqRows, err := s.db.QueryContext(c.Request.Context(), preqQuery, skillID)
	if err == nil {
		defer preqRows.Close()
		for preqRows.Next() {
			var preqCode string
			if err := preqRows.Scan(&preqCode); err == nil {
				skill.Prerequisites = append(skill.Prerequisites, preqCode)
			}
		}
	}

	c.JSON(http.StatusOK, skill)
}

// ============================================================================
// EXERCISE HANDLERS
// ============================================================================

// ListExercisesForSkill returns all exercises for a skill
func (s *ContentService) ListExercisesForSkill(c *gin.Context) {
	skillID := c.Param("skill_id")

	query := `
		SELECT id, skill_id, type, title, description, difficulty_delta,
		       estimated_time_seconds, usage_count, average_success_rate, is_published, created_at
		FROM exercises
		WHERE skill_id = $1 AND is_published = true
		ORDER BY created_at ASC
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query, skillID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch exercises",
		})
		return
	}
	defer rows.Close()

	exercises := []Exercise{}
	for rows.Next() {
		var exercise Exercise
		err := rows.Scan(
			&exercise.ID, &exercise.SkillID, &exercise.Type, &exercise.Title, &exercise.Description,
			&exercise.DifficultyDelta, &exercise.EstimatedTimeSeconds, &exercise.UsageCount,
			&exercise.AverageSuccessRate, &exercise.IsPublished, &exercise.CreatedAt,
		)
		if err != nil {
			continue
		}
		exercises = append(exercises, exercise)
	}

	if len(exercises) == 0 {
		exercises = []Exercise{}
	}

	c.JSON(http.StatusOK, gin.H{
		"exercises": exercises,
		"count":     len(exercises),
	})
}

// GetExercise returns a single exercise with full details
func (s *ContentService) GetExercise(c *gin.Context) {
	exerciseID := c.Param("exercise_id")

	query := `
		SELECT id, skill_id, type, title, description, difficulty_delta,
		       prompt, correct_option, options, explanation,
		       estimated_time_seconds, usage_count, average_success_rate,
		       is_published, created_at
		FROM exercises
		WHERE id = $1
	`

	var exercise Exercise
	var optionsJSON *string
	err := s.db.QueryRowContext(c.Request.Context(), query, exerciseID).Scan(
		&exercise.ID, &exercise.SkillID, &exercise.Type, &exercise.Title, &exercise.Description,
		&exercise.DifficultyDelta, &exercise.Prompt, &exercise.CorrectOption, &optionsJSON, &exercise.Explanation,
		&exercise.EstimatedTimeSeconds, &exercise.UsageCount, &exercise.AverageSuccessRate,
		&exercise.IsPublished, &exercise.CreatedAt,
	)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusNotFound, ErrorResponse{
			Error:   "NOT_FOUND",
			Message: "Exercise not found",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch exercise",
		})
		return
	}

	// Parse options JSON
	if optionsJSON != nil {
		json.Unmarshal([]byte(*optionsJSON), &exercise.Options)
	}

	c.JSON(http.StatusOK, exercise)
}

// ============================================================================
// LESSON HANDLERS
// ============================================================================

// ListLessonsForSkill returns all lessons for a skill
func (s *ContentService) ListLessonsForSkill(c *gin.Context) {
	skillID := c.Param("skill_id")

	query := `
		SELECT id, skill_id, sequence, title, description, created_at
		FROM lessons
		WHERE skill_id = $1
		ORDER BY sequence ASC
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query, skillID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch lessons",
		})
		return
	}
	defer rows.Close()

	lessons := []Lesson{}
	for rows.Next() {
		var lesson Lesson
		var objectives *string
		err := rows.Scan(
			&lesson.ID, &lesson.SkillID, &lesson.Sequence, &lesson.Title, &lesson.Description, &lesson.CreatedAt,
		)
		if err != nil {
			continue
		}

		// Parse objectives if present
		if objectives != nil {
			json.Unmarshal([]byte(*objectives), &lesson.LearningObjectives)
		}

		lessons = append(lessons, lesson)
	}

	if len(lessons) == 0 {
		lessons = []Lesson{}
	}

	c.JSON(http.StatusOK, gin.H{
		"lessons": lessons,
		"count":   len(lessons),
	})
}

// GetLesson returns a single lesson with exercises
func (s *ContentService) GetLesson(c *gin.Context) {
	lessonID := c.Param("lesson_id")

	var lesson Lesson
	query := `
		SELECT id, skill_id, sequence, title, description, created_at
		FROM lessons
		WHERE id = $1
	`

	err := s.db.QueryRowContext(c.Request.Context(), query, lessonID).Scan(
		&lesson.ID, &lesson.SkillID, &lesson.Sequence, &lesson.Title, &lesson.Description, &lesson.CreatedAt,
	)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusNotFound, ErrorResponse{
			Error:   "NOT_FOUND",
			Message: "Lesson not found",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch lesson",
		})
		return
	}

	// Fetch exercises for lesson
	exQuery := `
		SELECT e.id, e.skill_id, e.type, e.title, e.description, e.difficulty_delta,
		       e.estimated_time_seconds, e.usage_count, e.average_success_rate,
		       e.is_published, e.created_at
		FROM exercises e
		INNER JOIN lesson_exercises le ON e.id = le.exercise_id
		WHERE le.lesson_id = $1
		ORDER BY le.sequence ASC
	`

	exRows, err := s.db.QueryContext(c.Request.Context(), exQuery, lessonID)
	if err == nil {
		defer exRows.Close()
		for exRows.Next() {
			var exercise Exercise
			if err := exRows.Scan(
				&exercise.ID, &exercise.SkillID, &exercise.Type, &exercise.Title, &exercise.Description,
				&exercise.DifficultyDelta, &exercise.EstimatedTimeSeconds, &exercise.UsageCount,
				&exercise.AverageSuccessRate, &exercise.IsPublished, &exercise.CreatedAt,
			); err == nil {
				lesson.Exercises = append(lesson.Exercises, exercise)
			}
		}
	}

	c.JSON(http.StatusOK, lesson)
}

// ============================================================================
// CURRICULUM HANDLERS
// ============================================================================

// ListCurriculumPaths returns all published curriculum paths
func (s *ContentService) ListCurriculumPaths(c *gin.Context) {
	query := `
		SELECT id, code, name, description, language, is_published, created_at
		FROM curriculum_paths
		WHERE is_published = true
		ORDER BY created_at ASC
	`

	rows, err := s.db.QueryContext(c.Request.Context(), query)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch curriculum paths",
		})
		return
	}
	defer rows.Close()

	paths := []CurriculumPath{}
	for rows.Next() {
		var path CurriculumPath
		err := rows.Scan(
			&path.ID, &path.Code, &path.Name, &path.Description, &path.Language,
			&path.IsPublished, &path.CreatedAt,
		)
		if err != nil {
			continue
		}
		paths = append(paths, path)
	}

	if len(paths) == 0 {
		paths = []CurriculumPath{}
	}

	c.JSON(http.StatusOK, gin.H{
		"curriculum_paths": paths,
		"count":            len(paths),
	})
}

// GetCurriculumPath returns a curriculum path with all skills
func (s *ContentService) GetCurriculumPath(c *gin.Context) {
	pathID := c.Param("path_id")

	var path CurriculumPath
	query := `
		SELECT id, code, name, description, language, is_published, created_at
		FROM curriculum_paths
		WHERE id = $1
	`

	err := s.db.QueryRowContext(c.Request.Context(), query, pathID).Scan(
		&path.ID, &path.Code, &path.Name, &path.Description, &path.Language,
		&path.IsPublished, &path.CreatedAt,
	)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusNotFound, ErrorResponse{
			Error:   "NOT_FOUND",
			Message: "Curriculum path not found",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch curriculum path",
		})
		return
	}

	// Fetch skills for path
	skillQuery := `
		SELECT s.id, s.code, s.name, s.description, s.level, s.language, s.category,
		       s.icon_url, s.color_hex, s.estimated_time_minutes, s.difficulty_level,
		       s.is_published, s.created_at
		FROM skills s
		INNER JOIN curriculum_path_skills cps ON s.id = cps.skill_id
		WHERE cps.curriculum_path_id = $1
		ORDER BY cps.sequence ASC
	`

	skillRows, err := s.db.QueryContext(c.Request.Context(), skillQuery, pathID)
	if err == nil {
		defer skillRows.Close()
		for skillRows.Next() {
			var skill Skill
			if err := skillRows.Scan(
				&skill.ID, &skill.Code, &skill.Name, &skill.Description, &skill.Level,
				&skill.Language, &skill.Category, &skill.IconURL, &skill.ColorHex,
				&skill.EstimatedTimeMinutes, &skill.DifficultyLevel, &skill.IsPublished, &skill.CreatedAt,
			); err == nil {
				path.Skills = append(path.Skills, skill)
			}
		}
	}

	c.JSON(http.StatusOK, path)
}

// ============================================================================
// SEARCH HANDLER
// ============================================================================

// SearchContent searches skills and exercises by keyword
func (s *ContentService) SearchContent(c *gin.Context) {
	query := c.Query("q")
	if query == "" {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_INPUT",
			Message: "Search query required",
		})
		return
	}

	searchPattern := "%" + query + "%"

	// Search skills
	skillQuery := `
		SELECT id, code, name, description, level, language, category,
		       icon_url, color_hex, estimated_time_minutes, difficulty_level,
		       is_published, created_at
		FROM skills
		WHERE is_published = true
		  AND (name ILIKE $1 OR description ILIKE $1 OR code ILIKE $1)
		LIMIT 20
	`

	skillRows, _ := s.db.QueryContext(c.Request.Context(), skillQuery, searchPattern)
	defer skillRows.Close()

	skills := []Skill{}
	for skillRows.Next() {
		var skill Skill
		if err := skillRows.Scan(
			&skill.ID, &skill.Code, &skill.Name, &skill.Description, &skill.Level,
			&skill.Language, &skill.Category, &skill.IconURL, &skill.ColorHex,
			&skill.EstimatedTimeMinutes, &skill.DifficultyLevel, &skill.IsPublished, &skill.CreatedAt,
		); err == nil {
			skills = append(skills, skill)
		}
	}

	// Search exercises
	exerciseQuery := `
		SELECT id, skill_id, type, title, description, difficulty_delta,
		       estimated_time_seconds, usage_count, average_success_rate,
		       is_published, created_at
		FROM exercises
		WHERE is_published = true
		  AND (title ILIKE $1 OR description ILIKE $1)
		LIMIT 20
	`

	exerciseRows, _ := s.db.QueryContext(c.Request.Context(), exerciseQuery, searchPattern)
	defer exerciseRows.Close()

	exercises := []Exercise{}
	for exerciseRows.Next() {
		var exercise Exercise
		if err := exerciseRows.Scan(
			&exercise.ID, &exercise.SkillID, &exercise.Type, &exercise.Title, &exercise.Description,
			&exercise.DifficultyDelta, &exercise.EstimatedTimeSeconds, &exercise.UsageCount,
			&exercise.AverageSuccessRate, &exercise.IsPublished, &exercise.CreatedAt,
		); err == nil {
			exercises = append(exercises, exercise)
		}
	}

	c.JSON(http.StatusOK, gin.H{
		"skills":    skills,
		"exercises": exercises,
		"query":     query,
	})
}

// Health check
func (s *ContentService) Health(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "healthy",
		"service": "content-service",
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

	port := os.Getenv("CONTENT_SERVICE_PORT")
	if port == "" {
		port = "8002"
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
	service := NewContentService(db)

	// Setup router
	router := gin.Default()

	// Health check
	router.GET("/health", service.Health)

	// Skills
	router.GET("/v1/skills", service.ListSkills)
	router.GET("/v1/skills/:skill_id", service.GetSkill)

	// Exercises
	router.GET("/v1/skills/:skill_id/exercises", service.ListExercisesForSkill)
	router.GET("/v1/exercises/:exercise_id", service.GetExercise)

	// Lessons
	router.GET("/v1/skills/:skill_id/lessons", service.ListLessonsForSkill)
	router.GET("/v1/lessons/:lesson_id", service.GetLesson)

	// Curriculum
	router.GET("/v1/curriculum-paths", service.ListCurriculumPaths)
	router.GET("/v1/curriculum-paths/:path_id", service.GetCurriculumPath)

	// Search
	router.GET("/v1/search", service.SearchContent)

	// Start server
	log.Printf("Starting Content Service on port %s", port)
	if err := router.Run(":" + port); err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}
