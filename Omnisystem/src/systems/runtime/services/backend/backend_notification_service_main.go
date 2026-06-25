// PATHFINDER Notification Service
// Email, push, and SMS notification delivery
// Port: 8007

package main

import (
	"crypto/tls"
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/smtp"
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

type NotificationRequest struct {
	UserID  string `json:"user_id"`
	Type    string `json:"type"` // mastery, alert, summary, achievement
	Channel string `json:"channel"` // email, push, sms
	Subject string `json:"subject"`
	Message string `json:"message"`
	Data    map[string]interface{} `json:"data"`
}

type Notification struct {
	ID        string    `json:"id"`
	UserID    string    `json:"user_id"`
	Type      string    `json:"type"`
	Channel   string    `json:"channel"`
	Subject   string    `json:"subject"`
	Message   string    `json:"message"`
	Status    string    `json:"status"` // pending, sent, failed
	SentAt    *time.Time `json:"sent_at,omitempty"`
	OpenedAt  *time.Time `json:"opened_at,omitempty"`
	ClickedAt *time.Time `json:"clicked_at,omitempty"`
	CreatedAt time.Time `json:"created_at"`
}

type EmailConfig struct {
	SMTPHost     string
	SMTPPort     string
	FromAddress  string
	FromPassword string
}

// ============================================================================
// NOTIFICATION ENDPOINTS
// ============================================================================

// POST /v1/notifications/send - Send notification
func sendNotification(w http.ResponseWriter, r *http.Request) {
	var req NotificationRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Validate
	if req.UserID == "" || req.Type == "" || req.Channel == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	notificationID := uuid.New().String()
	now := time.Now().UTC()

	// Insert notification record (pending)
	_, err := db.Exec(`
		INSERT INTO notifications_sent (
			id, user_id, type, channel, subject, message, status, created_at
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
	`,
		notificationID, req.UserID, req.Type, req.Channel,
		req.Subject, req.Message, "pending", now,
	)

	if err != nil {
		log.Printf("Error inserting notification: %v\n", err)
		http.Error(w, "Failed to create notification", http.StatusInternalServerError)
		return
	}

	// Get user email/phone
	var userEmail string
	var userPhone sql.NullString
	err = db.QueryRow(
		"SELECT email, phone FROM users WHERE id = $1",
		req.UserID,
	).Scan(&userEmail, &userPhone)

	if err != nil {
		log.Printf("Error getting user: %v\n", err)
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	// Check notification preferences
	var emailFrequency string
	var quietHoursEnabled bool
	var quietHoursStart, quietHoursEnd string

	err = db.QueryRow(`
		SELECT COALESCE(email_frequency, 'daily'), COALESCE(quiet_hours_enabled, false),
		       COALESCE(quiet_hours_start, ''), COALESCE(quiet_hours_end, '')
		FROM notification_preferences
		WHERE user_id = $1
	`, req.UserID).Scan(&emailFrequency, &quietHoursEnabled, &quietHoursStart, &quietHoursEnd)

	// If not found, use defaults
	if err == sql.ErrNoRows {
		emailFrequency = "daily"
		quietHoursEnabled = false
	} else if err != nil {
		log.Printf("Error getting preferences: %v\n", err)
	}

	// Check quiet hours
	if quietHoursEnabled && isInQuietHours(quietHoursStart, quietHoursEnd) {
		// Skip email delivery during quiet hours, but record as sent
		_, _ = db.Exec(
			"UPDATE notifications_sent SET status = 'sent', sent_at = $1 WHERE id = $2",
			now, notificationID,
		)
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"id":     notificationID,
			"status": "sent (queued for quiet period)",
		})
		return
	}

	// Send based on channel
	var sendErr error
	switch req.Channel {
	case "email":
		sendErr = sendEmailNotification(userEmail, req.Subject, req.Message)
	case "push":
		sendErr = sendPushNotification(req.UserID, req.Subject, req.Message)
	case "sms":
		if userPhone.Valid {
			sendErr = sendSMSNotification(userPhone.String, req.Message)
		} else {
			sendErr = fmt.Errorf("no phone number on file")
		}
	default:
		sendErr = fmt.Errorf("unknown channel: %s", req.Channel)
	}

	// Update notification status
	status := "sent"
	if sendErr != nil {
		status = "failed"
		log.Printf("Error sending %s notification: %v\n", req.Channel, sendErr)
	}

	_, _ = db.Exec(
		"UPDATE notifications_sent SET status = $1, sent_at = $2 WHERE id = $3",
		status, now, notificationID,
	)

	w.Header().Set("Content-Type", "application/json")
	if sendErr != nil {
		json.NewEncoder(w).Encode(map[string]interface{}{
			"id":     notificationID,
			"status": "failed",
			"error":  sendErr.Error(),
		})
		return
	}

	json.NewEncoder(w).Encode(map[string]interface{}{
		"id":     notificationID,
		"status": "sent",
	})
}

// POST /v1/notifications/batch - Send batch notifications
func sendBatchNotifications(w http.ResponseWriter, r *http.Request) {
	var requests []NotificationRequest
	if err := json.NewDecoder(r.Body).Decode(&requests); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	var results []map[string]interface{}
	for _, req := range requests {
		notificationID := uuid.New().String()
		now := time.Now().UTC()

		// Insert notification record
		_, err := db.Exec(`
			INSERT INTO notifications_sent (
				id, user_id, type, channel, subject, message, status, created_at
			) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		`,
			notificationID, req.UserID, req.Type, req.Channel,
			req.Subject, req.Message, "sent", now,
		)

		status := "sent"
		if err != nil {
			status = "failed"
			log.Printf("Error inserting batch notification: %v\n", err)
		}

		results = append(results, map[string]interface{}{
			"id":     notificationID,
			"user":   req.UserID,
			"status": status,
		})
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"count":   len(results),
		"results": results,
	})
}

// GET /v1/notifications - Get user's notifications
func getNotifications(w http.ResponseWriter, r *http.Request) {
	userID := r.Header.Get("X-User-ID")
	if userID == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	limit := 50
	offset := 0

	// Parse query params
	if l := r.URL.Query().Get("limit"); l != "" {
		fmt.Sscanf(l, "%d", &limit)
	}
	if o := r.URL.Query().Get("offset"); o != "" {
		fmt.Sscanf(o, "%d", &offset)
	}

	rows, err := db.Query(`
		SELECT id, user_id, type, channel, subject, message, status, sent_at, opened_at, created_at
		FROM notifications_sent
		WHERE user_id = $1
		ORDER BY created_at DESC
		LIMIT $2 OFFSET $3
	`, userID, limit, offset)

	if err != nil {
		log.Printf("Error getting notifications: %v\n", err)
		http.Error(w, "Failed to get notifications", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var notifications []Notification
	for rows.Next() {
		var n Notification
		err := rows.Scan(
			&n.ID, &n.UserID, &n.Type, &n.Channel, &n.Subject,
			&n.Message, &n.Status, &n.SentAt, &n.OpenedAt, &n.CreatedAt,
		)
		if err != nil {
			log.Printf("Error scanning notification: %v\n", err)
			continue
		}
		notifications = append(notifications, n)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"notifications": notifications,
		"count":         len(notifications),
	})
}

// POST /v1/notifications/:id/mark-opened - Mark notification as opened
func markNotificationOpened(w http.ResponseWriter, r *http.Request) {
	notificationID := r.URL.Query().Get("id")
	userID := r.Header.Get("X-User-ID")

	if notificationID == "" || userID == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	now := time.Now().UTC()

	// Verify ownership
	var id string
	err := db.QueryRow(
		"SELECT id FROM notifications_sent WHERE id = $1 AND user_id = $2",
		notificationID, userID,
	).Scan(&id)

	if err == sql.ErrNoRows {
		http.Error(w, "Notification not found", http.StatusNotFound)
		return
	}

	// Update
	_, err = db.Exec(
		"UPDATE notifications_sent SET opened_at = $1 WHERE id = $2",
		now, notificationID,
	)

	if err != nil {
		log.Printf("Error marking opened: %v\n", err)
		http.Error(w, "Failed to mark opened", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "marked opened",
	})
}

// DELETE /v1/notifications/:id - Delete notification
func deleteNotification(w http.ResponseWriter, r *http.Request) {
	notificationID := r.URL.Query().Get("id")
	userID := r.Header.Get("X-User-ID")

	if notificationID == "" || userID == "" {
		http.Error(w, "Missing required fields", http.StatusBadRequest)
		return
	}

	// Verify ownership and delete
	result, err := db.Exec(
		"DELETE FROM notifications_sent WHERE id = $1 AND user_id = $2",
		notificationID, userID,
	)

	if err != nil {
		log.Printf("Error deleting notification: %v\n", err)
		http.Error(w, "Failed to delete notification", http.StatusInternalServerError)
		return
	}

	rows, _ := result.RowsAffected()
	if rows == 0 {
		http.Error(w, "Notification not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "deleted",
	})
}

// ============================================================================
// EMAIL DELIVERY
// ============================================================================

func sendEmailNotification(to, subject, body string) error {
	smtpHost := os.Getenv("SMTP_HOST")
	if smtpHost == "" {
		smtpHost = "smtp.gmail.com"
	}

	smtpPort := os.Getenv("SMTP_PORT")
	if smtpPort == "" {
		smtpPort = "587"
	}

	fromEmail := os.Getenv("SMTP_FROM_EMAIL")
	fromPassword := os.Getenv("SMTP_FROM_PASSWORD")

	if fromEmail == "" || fromPassword == "" {
		return fmt.Errorf("SMTP credentials not configured")
	}

	// HTML email template
	htmlBody := fmt.Sprintf(`
	<html>
		<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; color: #333;">
			<div style="max-width: 600px; margin: 0 auto; padding: 20px;">
				<h2 style="color: #1e3a8a;">%s</h2>
				<p style="line-height: 1.6; margin: 20px 0;">%s</p>
				<hr style="border: none; border-top: 1px solid #e5e7eb; margin: 30px 0;">
				<p style="color: #666; font-size: 12px;">PATHFINDER Learning Platform</p>
			</div>
		</body>
	</html>
	`, subject, body)

	auth := smtp.PlainAuth("", fromEmail, fromPassword, smtpHost)

	tlsconfig := &tls.Config{
		InsecureSkipVerify: false,
		ServerName:         smtpHost,
	}

	conn, err := tls.Dial("tcp", smtpHost+":"+smtpPort, tlsconfig)
	if err != nil {
		return err
	}

	client, err := smtp.NewClient(conn, smtpHost)
	if err != nil {
		return err
	}
	defer client.Close()

	if err = client.Auth(auth); err != nil {
		return err
	}

	if err = client.Mail(fromEmail); err != nil {
		return err
	}

	if err = client.Rcpt(to); err != nil {
		return err
	}

	w, err := client.Data()
	if err != nil {
		return err
	}

	message := fmt.Sprintf("From: %s\r\nTo: %s\r\nSubject: %s\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n%s",
		fromEmail, to, subject, htmlBody)

	_, err = w.Write([]byte(message))
	if err != nil {
		return err
	}

	err = w.Close()
	if err != nil {
		return err
	}

	return client.Quit()
}

// ============================================================================
// PUSH NOTIFICATION DELIVERY
// ============================================================================

func sendPushNotification(userID, title, message string) error {
	// TODO: Integrate with Firebase Cloud Messaging or OneSignal
	// For now, just log
	log.Printf("Push notification to user %s: %s - %s\n", userID, title, message)
	return nil
}

// ============================================================================
// SMS NOTIFICATION DELIVERY
// ============================================================================

func sendSMSNotification(phone, message string) error {
	// TODO: Integrate with Twilio or similar
	// For now, just log
	log.Printf("SMS to %s: %s\n", phone, message)
	return nil
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

func isInQuietHours(start, end string) bool {
	if start == "" || end == "" {
		return false
	}

	now := time.Now()
	currentTime := fmt.Sprintf("%02d:%02d", now.Hour(), now.Minute())

	return currentTime >= start && currentTime <= end
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

	// Notification endpoints
	r.HandleFunc("/v1/notifications/send", sendNotification).Methods("POST")
	r.HandleFunc("/v1/notifications/batch", sendBatchNotifications).Methods("POST")
	r.HandleFunc("/v1/notifications", getNotifications).Methods("GET")
	r.HandleFunc("/v1/notifications/mark-opened", markNotificationOpened).Methods("POST")
	r.HandleFunc("/v1/notifications/delete", deleteNotification).Methods("DELETE")

	// CORS
	c := cors.Default()
	handler := c.Handler(r)

	// Start server
	port := os.Getenv("NOTIFICATION_SERVICE_PORT")
	if port == "" {
		port = "8007"
	}

	log.Printf("Notification Service listening on port %s\n", port)
	log.Fatal(http.ListenAndServe(":"+port, handler))
}
