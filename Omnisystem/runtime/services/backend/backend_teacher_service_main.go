// PATHFINDER Teacher Service
// Classroom management, student monitoring, analytics
// Port: 8005

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

type Classroom struct {
	ID          string                 `json:"id"`
	TeacherID   string                 `json:"teacher_id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Subject     string                 `json:"subject"`
	GradeLevel  string                 `json:"grade_level"`
	Capacity    int                    `json:"capacity"`
	InviteCode  string                 `json:"invite_code"`
	Settings    map[string]interface{} `json:"settings"`
	CreatedAt   string                 `json:"created_at"`
	UpdatedAt   string                 `json:"updated_at"`
}

type ClassroomStudent struct {
	StudentID        string  `json:"student_id"`
	Name             string  `json:"name"`
	Email            string  `json:"email"`
	MasteryPercent   float64 `json:"mastery_percent"`
	SkillsMastered   int     `json:"skills_mastered"`
	TotalSkills      int     `json:"total_skills"`
	CurrentSkill     string  `json:"current_skill"`
	LastActivity     string  `json:"last_activity"`
	Status           string  `json:"status"` // active, inactive, struggling
	JoinedAt         string  `json:"joined_at"`
}

type InterventionAlert struct {
	ID               string `json:"id"`
	ClassroomID      string `json:"classroom_id"`
	StudentID        string `json:"student_id"`
	StudentName      string `json:"student_name"`
	AlertType        string `json:"alert_type"` // struggling, falling_behind, inactive
	SkillID          string `json:"skill_id"`
	SkillName        string `json:"skill_name"`
	Message          string `json:"message"`
	Severity         string `json:"severity"` // low, medium, high
	ProbabilityKnow  float64 `json:"p_know"`
	DaysSinceProgress int    `json:"days_since_progress"`
	Recommendation   string `json:"recommendation"`
	Resolved         bool   `json:"resolved"`
	CreatedAt        string `json:"created_at"`
	ResolvedAt       string `json:"resolved_at"`
}

type ClassroomProgress struct {
	ClassroomID            string                   `json:"classroom_id"`
	TotalStudents          int                      `json:"total_students"`
	ActiveStudents         int                      `json:"active_students"`
	AvgMastery             float64                  `json:"avg_mastery"`
	MasteryDistribution    map[string]int           `json:"mastery_distribution"`
	TopSkills              []SkillStat              `json:"top_skills"`
	StrugglingSkills       []SkillStat              `json:"struggling_skills"`
	Engagement             map[string]interface{}   `json:"engagement"`
}

type SkillStat struct {
	SkillID           string  `json:"skill_id"`
	SkillName         string  `json:"skill_name"`
	MasteryPercent    float64 `json:"mastery_percent"`
	StudentsMastered  int     `json:"students_mastered"`
	StudentsStruggling int     `json:"students_struggling"`
}

// ============================================================================
// CLASSROOM ENDPOINTS
// ============================================================================

// POST /v1/teachers/classrooms - Create classroom
func createClassroom(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Name       string `json:"name"`
		Description string `json:"description"`
		Subject    string `json:"subject"`
		GradeLevel string `json:"grade_level"`
		Capacity   int    `json:"capacity"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Validate
	if req.Name == "" || req.Subject == "" || req.GradeLevel == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	if req.Capacity <= 0 || req.Capacity > 100 {
		req.Capacity = 30
	}

	teacherID := r.Header.Get("X-User-ID")
	if teacherID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	classroomID := uuid.New().String()
	inviteCode := generateInviteCode()
	now := time.Now().UTC().Format(time.RFC3339)
	settings := map[string]interface{}{
		"allow_peer_learning":  true,
		"show_leaderboard":     true,
		"parent_access":        true,
		"mastery_threshold":    0.85,
	}
	settingsJSON, _ := json.Marshal(settings)

	query := `
	INSERT INTO classrooms (
		id, teacher_id, name, description, subject, grade_level,
		capacity, invite_code, settings, created_at, updated_at
	) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
	`

	_, err := db.Exec(query,
		classroomID, teacherID, req.Name, req.Description,
		req.Subject, req.GradeLevel, req.Capacity, inviteCode,
		settingsJSON, now, now,
	)

	if err != nil {
		log.Printf("Error creating classroom: %v\n", err)
		http.Error(w, "Failed to create classroom", http.StatusInternalServerError)
		return
	}

	classroom := Classroom{
		ID:          classroomID,
		TeacherID:   teacherID,
		Name:        req.Name,
		Description: req.Description,
		Subject:     req.Subject,
		GradeLevel:  req.GradeLevel,
		Capacity:    req.Capacity,
		InviteCode:  inviteCode,
		Settings:    settings,
		CreatedAt:   now,
		UpdatedAt:   now,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(classroom)
}

// GET /v1/teachers/classrooms - List teacher's classrooms
func listClassrooms(w http.ResponseWriter, r *http.Request) {
	teacherID := r.Header.Get("X-User-ID")
	if teacherID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	SELECT id, teacher_id, name, description, subject, grade_level,
	       capacity, invite_code, settings, created_at, updated_at
	FROM classrooms
	WHERE teacher_id = $1
	ORDER BY created_at DESC
	`

	rows, err := db.Query(query, teacherID)
	if err != nil {
		log.Printf("Error listing classrooms: %v\n", err)
		http.Error(w, "Failed to list classrooms", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var classrooms []Classroom
	for rows.Next() {
		var c Classroom
		var settingsJSON []byte

		err := rows.Scan(
			&c.ID, &c.TeacherID, &c.Name, &c.Description,
			&c.Subject, &c.GradeLevel, &c.Capacity, &c.InviteCode,
			&settingsJSON, &c.CreatedAt, &c.UpdatedAt,
		)
		if err != nil {
			log.Printf("Error scanning classroom: %v\n", err)
			continue
		}

		json.Unmarshal(settingsJSON, &c.Settings)
		classrooms = append(classrooms, c)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"classrooms": classrooms,
	})
}

// GET /v1/teachers/classrooms/:id - Get classroom details
func getClassroom(w http.ResponseWriter, r *http.Request) {
	classroomID := mux.Vars(r)["id"]
	teacherID := r.Header.Get("X-User-ID")

	query := `
	SELECT id, teacher_id, name, description, subject, grade_level,
	       capacity, invite_code, settings, created_at, updated_at
	FROM classrooms
	WHERE id = $1 AND teacher_id = $2
	`

	var c Classroom
	var settingsJSON []byte

	err := db.QueryRow(query, classroomID, teacherID).Scan(
		&c.ID, &c.TeacherID, &c.Name, &c.Description,
		&c.Subject, &c.GradeLevel, &c.Capacity, &c.InviteCode,
		&settingsJSON, &c.CreatedAt, &c.UpdatedAt,
	)

	if err == sql.ErrNoRows {
		http.Error(w, "Classroom not found", http.StatusNotFound)
		return
	}
	if err != nil {
		log.Printf("Error getting classroom: %v\n", err)
		http.Error(w, "Failed to get classroom", http.StatusInternalServerError)
		return
	}

	json.Unmarshal(settingsJSON, &c.Settings)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(c)
}

// ============================================================================
// STUDENT ROSTER ENDPOINTS
// ============================================================================

// POST /v1/teachers/classrooms/:id/students - Add student
func addStudent(w http.ResponseWriter, r *http.Request) {
	classroomID := mux.Vars(r)["id"]
	teacherID := r.Header.Get("X-User-ID")

	var req struct {
		StudentID string `json:"student_id"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Verify classroom belongs to teacher
	var owner string
	db.QueryRow("SELECT teacher_id FROM classrooms WHERE id = $1", classroomID).Scan(&owner)
	if owner != teacherID {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	query := `
	INSERT INTO classroom_students (id, classroom_id, student_id, joined_at)
	VALUES ($1, $2, $3, $4)
	ON CONFLICT DO NOTHING
	`

	_, err := db.Exec(query,
		uuid.New().String(),
		classroomID,
		req.StudentID,
		time.Now().UTC().Format(time.RFC3339),
	)

	if err != nil {
		log.Printf("Error adding student: %v\n", err)
		http.Error(w, "Failed to add student", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "student added",
	})
}

// GET /v1/teachers/classrooms/:id/students - List students
func listStudents(w http.ResponseWriter, r *http.Request) {
	classroomID := mux.Vars(r)["id"]
	teacherID := r.Header.Get("X-User-ID")

	// Verify classroom belongs to teacher
	var owner string
	err := db.QueryRow("SELECT teacher_id FROM classrooms WHERE id = $1", classroomID).Scan(&owner)
	if err != nil || owner != teacherID {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get student roster with progress
	query := `
	SELECT u.id, u.first_name || ' ' || u.last_name as name, u.email,
	       COALESCE(lp.mastery_percentage, 0) as mastery_percent,
	       COALESCE(lp.mastered_skills, 0) as skills_mastered,
	       COALESCE(lp.total_skills, 0) as total_skills,
	       COALESCE(lp.current_skill, '') as current_skill,
	       COALESCE(MAX(ea.created_at), '') as last_activity,
	       CASE
	           WHEN COALESCE(lp.mastery_percentage, 0) >= 0.85 THEN 'active'
	           WHEN COALESCE(lp.mastery_percentage, 0) < 0.30 THEN 'struggling'
	           ELSE 'active'
	       END as status,
	       cs.joined_at
	FROM classroom_students cs
	JOIN users u ON cs.student_id = u.id
	LEFT JOIN learner_progress lp ON cs.student_id = lp.user_id
	LEFT JOIN exercise_attempts ea ON cs.student_id = ea.user_id
	WHERE cs.classroom_id = $1
	GROUP BY u.id, u.first_name, u.last_name, u.email, lp.mastery_percentage,
	         lp.mastered_skills, lp.total_skills, lp.current_skill, cs.joined_at
	ORDER BY u.first_name, u.last_name
	`

	rows, err := db.Query(query, classroomID)
	if err != nil {
		log.Printf("Error listing students: %v\n", err)
		http.Error(w, "Failed to list students", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var students []ClassroomStudent
	for rows.Next() {
		var s ClassroomStudent
		err := rows.Scan(
			&s.StudentID, &s.Name, &s.Email, &s.MasteryPercent,
			&s.SkillsMastered, &s.TotalSkills, &s.CurrentSkill,
			&s.LastActivity, &s.Status, &s.JoinedAt,
		)
		if err != nil {
			log.Printf("Error scanning student: %v\n", err)
			continue
		}
		s.MasteryPercent = s.MasteryPercent * 100 // Convert to percentage
		students = append(students, s)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"students": students,
		"count":    len(students),
	})
}

// DELETE /v1/teachers/classrooms/:id/students/:uid - Remove student
func removeStudent(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	classroomID := vars["id"]
	studentID := vars["uid"]
	teacherID := r.Header.Get("X-User-ID")

	// Verify classroom belongs to teacher
	var owner string
	db.QueryRow("SELECT teacher_id FROM classrooms WHERE id = $1", classroomID).Scan(&owner)
	if owner != teacherID {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	_, err := db.Exec(
		"DELETE FROM classroom_students WHERE classroom_id = $1 AND student_id = $2",
		classroomID, studentID,
	)

	if err != nil {
		log.Printf("Error removing student: %v\n", err)
		http.Error(w, "Failed to remove student", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "student removed",
	})
}

// ============================================================================
// ANALYTICS ENDPOINTS
// ============================================================================

// GET /v1/teachers/classrooms/:id/progress - Class progress overview
func getClassProgress(w http.ResponseWriter, r *http.Request) {
	classroomID := mux.Vars(r)["id"]
	teacherID := r.Header.Get("X-User-ID")

	// Verify ownership
	var owner string
	db.QueryRow("SELECT teacher_id FROM classrooms WHERE id = $1", classroomID).Scan(&owner)
	if owner != teacherID {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	progress := ClassroomProgress{
		ClassroomID:         classroomID,
		MasteryDistribution: make(map[string]int),
		Engagement:          make(map[string]interface{}),
	}

	// Get overall stats
	statsQuery := `
	SELECT COUNT(DISTINCT cs.student_id) as total,
	       COUNT(DISTINCT CASE WHEN ea.created_at > NOW() - INTERVAL '7 days' THEN cs.student_id END) as active,
	       AVG(COALESCE(lp.mastery_percentage, 0)) as avg_mastery
	FROM classroom_students cs
	LEFT JOIN learner_progress lp ON cs.student_id = lp.user_id
	LEFT JOIN exercise_attempts ea ON cs.student_id = ea.user_id
	WHERE cs.classroom_id = $1
	`

	var avgMastery sql.NullFloat64
	db.QueryRow(statsQuery, classroomID).Scan(
		&progress.TotalStudents,
		&progress.ActiveStudents,
		&avgMastery,
	)

	if avgMastery.Valid {
		progress.AvgMastery = avgMastery.Float64 * 100
	}

	// Get mastery distribution
	progress.MasteryDistribution["mastered"] = 0
	progress.MasteryDistribution["developing"] = 0
	progress.MasteryDistribution["beginner"] = 0

	distQuery := `
	SELECT
	  SUM(CASE WHEN lp.mastery_percentage >= 0.85 THEN 1 ELSE 0 END) as mastered,
	  SUM(CASE WHEN lp.mastery_percentage BETWEEN 0.30 AND 0.85 THEN 1 ELSE 0 END) as developing,
	  SUM(CASE WHEN lp.mastery_percentage < 0.30 THEN 1 ELSE 0 END) as beginner
	FROM classroom_students cs
	LEFT JOIN learner_progress lp ON cs.student_id = lp.user_id
	WHERE cs.classroom_id = $1
	`

	var mastered, developing, beginner sql.NullInt64
	db.QueryRow(distQuery, classroomID).Scan(&mastered, &developing, &beginner)

	if mastered.Valid {
		progress.MasteryDistribution["mastered"] = int(mastered.Int64)
	}
	if developing.Valid {
		progress.MasteryDistribution["developing"] = int(developing.Int64)
	}
	if beginner.Valid {
		progress.MasteryDistribution["beginner"] = int(beginner.Int64)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(progress)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

func generateInviteCode() string {
	return uuid.New().String()[:8]
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

	// Classroom endpoints
	r.HandleFunc("/v1/teachers/classrooms", createClassroom).Methods("POST")
	r.HandleFunc("/v1/teachers/classrooms", listClassrooms).Methods("GET")
	r.HandleFunc("/v1/teachers/classrooms/{id}", getClassroom).Methods("GET")

	// Student roster endpoints
	r.HandleFunc("/v1/teachers/classrooms/{id}/students", addStudent).Methods("POST")
	r.HandleFunc("/v1/teachers/classrooms/{id}/students", listStudents).Methods("GET")
	r.HandleFunc("/v1/teachers/classrooms/{id}/students/{uid}", removeStudent).Methods("DELETE")

	// Analytics endpoints
	r.HandleFunc("/v1/teachers/classrooms/{id}/progress", getClassProgress).Methods("GET")

	// CORS
	c := cors.Default()
	handler := c.Handler(r)

	// Start server
	port := os.Getenv("TEACHER_SERVICE_PORT")
	if port == "" {
		port = "8005"
	}

	log.Printf("Teacher Service listening on port %s\n", port)
	log.Fatal(http.ListenAndServe(":"+port, handler))
}
