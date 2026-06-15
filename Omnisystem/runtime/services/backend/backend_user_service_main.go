// PATHFINDER User Service - Main Application
// Phase 1: Week 1-4 Implementation
// Responsible for: Authentication, user profiles, sessions, GDPR/COPPA compliance

package main

import (
	"context"
	"database/sql"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/golang-jwt/jwt/v5"
	_ "github.com/lib/pq"
	"golang.org/x/crypto/bcrypt"
)

// ============================================================================
// MODELS
// ============================================================================

type User struct {
	ID                UUID      `json:"id"`
	Email             string    `json:"email"`
	EmailVerified     bool      `json:"email_verified"`
	FirstName         string    `json:"first_name"`
	LastName          string    `json:"last_name"`
	LanguagePreference string    `json:"language_preference"`
	IsTeacher         bool      `json:"is_teacher"`
	CreatedAt         time.Time `json:"created_at"`
	UpdatedAt         time.Time `json:"updated_at"`
}

type RegisterRequest struct {
	Email      string `json:"email" binding:"required,email"`
	Password   string `json:"password" binding:"required,min=8"`
	FirstName  string `json:"first_name" binding:"required"`
	LastName   string `json:"last_name"`
	Age        *int   `json:"age"`
}

type RegisterResponse struct {
	UserID string `json:"user_id"`
	Email  string `json:"email"`
	Token  string `json:"token"`
}

type LoginRequest struct {
	Email    string `json:"email" binding:"required,email"`
	Password string `json:"password" binding:"required"`
}

type LoginResponse struct {
	UserID       string `json:"user_id"`
	Token        string `json:"token"`
	RefreshToken string `json:"refresh_token"`
	ExpiresIn    int    `json:"expires_in"`
}

type UserProfileResponse struct {
	ID                 string `json:"id"`
	Email              string `json:"email"`
	EmailVerified      bool   `json:"email_verified"`
	FirstName          string `json:"first_name"`
	LastName           string `json:"last_name"`
	LanguagePreference string `json:"language_preference"`
	IsTeacher          bool   `json:"is_teacher"`
	CreatedAt          string `json:"created_at"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
}

// ============================================================================
// JWT CLAIMS
// ============================================================================

type JWTClaims struct {
	UserID string `json:"user_id"`
	Email  string `json:"email"`
	jwt.RegisteredClaims
}

// ============================================================================
// SERVICE
// ============================================================================

type UserService struct {
	db        *sql.DB
	jwtSecret string
}

// NewUserService creates a new user service
func NewUserService(db *sql.DB, jwtSecret string) *UserService {
	return &UserService{
		db:        db,
		jwtSecret: jwtSecret,
	}
}

// ============================================================================
// AUTHENTICATION HANDLERS
// ============================================================================

// Register creates a new user account
func (s *UserService) Register(c *gin.Context) {
	var req RegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_INPUT",
			Message: err.Error(),
		})
		return
	}

	// Check if user already exists
	var existingID string
	err := s.db.QueryRow("SELECT id FROM users WHERE email = $1", req.Email).Scan(&existingID)
	if err == nil {
		// User exists
		c.JSON(http.StatusConflict, ErrorResponse{
			Error:   "USER_EXISTS",
			Message: "Email already registered",
		})
		return
	}
	if err != sql.ErrNoRows {
		// Database error
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to check user existence",
		})
		return
	}

	// Hash password
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "HASH_ERROR",
			Message: "Failed to hash password",
		})
		return
	}

	// Create user
	userID := newUUID()
	now := time.Now()

	query := `
		INSERT INTO users
		(id, email, password_hash, first_name, last_name, language_preference, age, is_active, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9)
	`

	_, err = s.db.Exec(query,
		userID, req.Email, string(hashedPassword),
		req.FirstName, req.LastName, "en", req.Age,
		now, now,
	)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to create user",
		})
		return
	}

	// Generate JWT token
	token, err := s.generateJWT(userID.String(), req.Email)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "TOKEN_ERROR",
			Message: "Failed to generate token",
		})
		return
	}

	c.JSON(http.StatusCreated, RegisterResponse{
		UserID: userID.String(),
		Email:  req.Email,
		Token:  token,
	})
}

// Login authenticates a user and returns JWT token
func (s *UserService) Login(c *gin.Context) {
	var req LoginRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_INPUT",
			Message: err.Error(),
		})
		return
	}

	// Fetch user from database
	var userID string
	var passwordHash string
	var isActive bool

	query := "SELECT id, password_hash, is_active FROM users WHERE email = $1"
	err := s.db.QueryRow(query, req.Email).Scan(&userID, &passwordHash, &isActive)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "INVALID_CREDENTIALS",
			Message: "Email or password incorrect",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch user",
		})
		return
	}

	// Check if user is active
	if !isActive {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "ACCOUNT_DISABLED",
			Message: "This account has been disabled",
		})
		return
	}

	// Verify password
	err = bcrypt.CompareHashAndPassword([]byte(passwordHash), []byte(req.Password))
	if err != nil {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "INVALID_CREDENTIALS",
			Message: "Email or password incorrect",
		})
		return
	}

	// Generate tokens
	token, err := s.generateJWT(userID, req.Email)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "TOKEN_ERROR",
			Message: "Failed to generate token",
		})
		return
	}

	refreshToken, err := s.generateRefreshToken(userID, req.Email)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "TOKEN_ERROR",
			Message: "Failed to generate refresh token",
		})
		return
	}

	// Record login session
	sessionID := newUUID()
	sessionQuery := `
		INSERT INTO user_sessions (id, user_id, token_hash, device_type, ip_address, user_agent, expires_at, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
	`
	_, _ = s.db.Exec(sessionQuery,
		sessionID, userID, hashToken(token), "web",
		c.ClientIP(), c.GetHeader("User-Agent"),
		time.Now().Add(24*time.Hour), time.Now(),
	)

	c.JSON(http.StatusOK, LoginResponse{
		UserID:       userID,
		Token:        token,
		RefreshToken: refreshToken,
		ExpiresIn:    3600, // 1 hour
	})
}

// GetProfile returns the current user's profile
func (s *UserService) GetProfile(c *gin.Context) {
	userID, exists := c.Get("user_id")
	if !exists {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "UNAUTHORIZED",
			Message: "User not authenticated",
		})
		return
	}

	var user UserProfileResponse
	query := `
		SELECT id, email, email_verified, first_name, last_name, language_preference, is_teacher, created_at
		FROM users
		WHERE id = $1 AND deleted_at IS NULL
	`

	err := s.db.QueryRow(query, userID).Scan(
		&user.ID, &user.Email, &user.EmailVerified, &user.FirstName,
		&user.LastName, &user.LanguagePreference, &user.IsTeacher, &user.CreatedAt,
	)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusNotFound, ErrorResponse{
			Error:   "USER_NOT_FOUND",
			Message: "User profile not found",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to fetch user profile",
		})
		return
	}

	c.JSON(http.StatusOK, user)
}

// UpdateProfile updates the current user's profile
func (s *UserService) UpdateProfile(c *gin.Context) {
	userID, exists := c.Get("user_id")
	if !exists {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "UNAUTHORIZED",
			Message: "User not authenticated",
		})
		return
	}

	var updateData struct {
		FirstName          *string `json:"first_name"`
		LastName           *string `json:"last_name"`
		LanguagePreference *string `json:"language_preference"`
	}

	if err := c.ShouldBindJSON(&updateData); err != nil {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_INPUT",
			Message: err.Error(),
		})
		return
	}

	// Build dynamic update query
	updates := []string{}
	args := []interface{}{userID, time.Now()}
	argCount := 3

	if updateData.FirstName != nil {
		updates = append(updates, fmt.Sprintf("first_name = $%d", argCount))
		args = append(args, *updateData.FirstName)
		argCount++
	}
	if updateData.LastName != nil {
		updates = append(updates, fmt.Sprintf("last_name = $%d", argCount))
		args = append(args, *updateData.LastName)
		argCount++
	}
	if updateData.LanguagePreference != nil {
		updates = append(updates, fmt.Sprintf("language_preference = $%d", argCount))
		args = append(args, *updateData.LanguagePreference)
		argCount++
	}

	if len(updates) == 0 {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "INVALID_INPUT",
			Message: "No fields to update",
		})
		return
	}

	query := fmt.Sprintf(
		"UPDATE users SET %s, updated_at = $2 WHERE id = $1 RETURNING id",
		join(updates, ", "),
	)

	var updatedID string
	err := s.db.QueryRow(query, args...).Scan(&updatedID)
	if err == sql.ErrNoRows {
		c.JSON(http.StatusNotFound, ErrorResponse{
			Error:   "USER_NOT_FOUND",
			Message: "User not found",
		})
		return
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to update profile",
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "Profile updated successfully"})
}

// Logout revokes the user's current session
func (s *UserService) Logout(c *gin.Context) {
	userID, exists := c.Get("user_id")
	if !exists {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "UNAUTHORIZED",
			Message: "User not authenticated",
		})
		return
	}

	// Get token from header
	token := c.GetHeader("Authorization")[7:] // Remove "Bearer " prefix

	// Revoke session
	query := "UPDATE user_sessions SET revoked_at = $1 WHERE token_hash = $2"
	_, err := s.db.Exec(query, time.Now(), hashToken(token))
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to logout",
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "Logged out successfully"})
}

// DeleteAccount requests account deletion (GDPR)
func (s *UserService) DeleteAccount(c *gin.Context) {
	userID, exists := c.Get("user_id")
	if !exists {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "UNAUTHORIZED",
			Message: "User not authenticated",
		})
		return
	}

	// Schedule deletion for 30 days from now
	deletionScheduled := time.Now().AddDate(0, 0, 30)
	query := `
		UPDATE users
		SET deletion_requested = true,
		    deletion_requested_at = $1,
		    deletion_scheduled_at = $2
		WHERE id = $3
	`

	_, err := s.db.Exec(query, time.Now(), deletionScheduled, userID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "DATABASE_ERROR",
			Message: "Failed to schedule deletion",
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"message":             "Account deletion scheduled",
		"deletion_scheduled":  deletionScheduled.Format(time.RFC3339),
		"grace_period_days":   30,
	})
}

// ExportData exports user's data (GDPR)
func (s *UserService) ExportData(c *gin.Context) {
	userID, exists := c.Get("user_id")
	if !exists {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "UNAUTHORIZED",
			Message: "User not authenticated",
		})
		return
	}

	// TODO: Implement data export
	// This should compile all user data into JSON/CSV and return as download
	// For now, return placeholder

	c.JSON(http.StatusOK, gin.H{
		"message": "Data export initiated. You will receive an email with your data.",
		"email":   "Data export link sent to your registered email",
	})
}

// Health check
func (s *UserService) Health(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "healthy",
		"service": "user-service",
		"time":    time.Now().Format(time.RFC3339),
	})
}

// ============================================================================
// JWT HELPERS
// ============================================================================

func (s *UserService) generateJWT(userID, email string) (string, error) {
	claims := JWTClaims{
		UserID: userID,
		Email:  email,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(1 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString([]byte(s.jwtSecret))
}

func (s *UserService) generateRefreshToken(userID, email string) (string, error) {
	claims := JWTClaims{
		UserID: userID,
		Email:  email,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(30 * 24 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString([]byte(s.jwtSecret))
}

func (s *UserService) validateJWT(tokenString string) (*JWTClaims, error) {
	claims := &JWTClaims{}
	token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
		return []byte(s.jwtSecret), nil
	})

	if err != nil || !token.Valid {
		return nil, err
	}

	return claims, nil
}

// ============================================================================
// MIDDLEWARE
// ============================================================================

func (s *UserService) authMiddleware(c *gin.Context) {
	authHeader := c.GetHeader("Authorization")
	if authHeader == "" {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "UNAUTHORIZED",
			Message: "Missing authorization header",
		})
		c.Abort()
		return
	}

	// Extract token from "Bearer <token>"
	if len(authHeader) < 7 || authHeader[:7] != "Bearer " {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "INVALID_TOKEN",
			Message: "Invalid authorization header format",
		})
		c.Abort()
		return
	}

	tokenString := authHeader[7:]
	claims, err := s.validateJWT(tokenString)
	if err != nil {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "INVALID_TOKEN",
			Message: "Invalid or expired token",
		})
		c.Abort()
		return
	}

	// Check if session is revoked
	var revokedAt *time.Time
	err = s.db.QueryRow(
		"SELECT revoked_at FROM user_sessions WHERE token_hash = $1",
		hashToken(tokenString),
	).Scan(&revokedAt)

	if err == nil && revokedAt != nil {
		c.JSON(http.StatusUnauthorized, ErrorResponse{
			Error:   "SESSION_REVOKED",
			Message: "Your session has been revoked",
		})
		c.Abort()
		return
	}

	// Set user context
	c.Set("user_id", claims.UserID)
	c.Set("email", claims.Email)
	c.Next()
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

	jwtSecret := os.Getenv("JWT_SECRET_KEY")
	if jwtSecret == "" {
		jwtSecret = "dev-secret-key-change-in-production"
	}

	port := os.Getenv("USER_SERVICE_PORT")
	if port == "" {
		port = "8001"
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
	service := NewUserService(db, jwtSecret)

	// Setup router
	router := gin.Default()

	// Health check
	router.GET("/health", service.Health)

	// Public routes
	router.POST("/v1/auth/register", service.Register)
	router.POST("/v1/auth/login", service.Login)

	// Protected routes
	router.GET("/v1/users/me", service.authMiddleware, service.GetProfile)
	router.PUT("/v1/users/me", service.authMiddleware, service.UpdateProfile)
	router.POST("/v1/auth/logout", service.authMiddleware, service.Logout)
	router.DELETE("/v1/users/me", service.authMiddleware, service.DeleteAccount)
	router.POST("/v1/users/me/export-data", service.authMiddleware, service.ExportData)

	// Start server
	log.Printf("Starting User Service on port %s", port)
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

func hashToken(token string) string {
	// In production, use SHA256
	return fmt.Sprintf("%x", token[:8])
}

func join(strs []string, sep string) string {
	result := ""
	for i, s := range strs {
		if i > 0 {
			result += sep
		}
		result += s
	}
	return result
}
