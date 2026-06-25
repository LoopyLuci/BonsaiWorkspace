// PATHFINDER Integration Test Suite
// Comprehensive testing for all microservices

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"testing"
	"time"
)

const (
	baseURL = "http://localhost:8001"
	timeout = 10 * time.Second
)

var (
	testUserID       = "test_user_123"
	testToken        = "test_token_xyz"
	testClassroomID  = "classroom_456"
	testSkillID      = "skill_789"
	testExerciseID   = "ex_100"
)

type TestClient struct {
	client *http.Client
	token  string
}

func NewTestClient(token string) *TestClient {
	return &TestClient{
		client: &http.Client{Timeout: timeout},
		token:  token,
	}
}

func (tc *TestClient) Do(method, path string, body interface{}) (int, []byte, error) {
	url := fmt.Sprintf("%s/v1%s", baseURL, path)

	var reqBody io.Reader
	if body != nil {
		jsonBody, _ := json.Marshal(body)
		reqBody = bytes.NewBuffer(jsonBody)
	}

	req, _ := http.NewRequest(method, url, reqBody)
	req.Header.Set("X-User-ID", testUserID)
	req.Header.Set("Authorization", "Bearer "+tc.token)
	req.Header.Set("Content-Type", "application/json")

	resp, err := tc.client.Do(req)
	if err != nil {
		return 0, nil, err
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, respBody, nil
}

// ========================
// USER SERVICE TESTS
// ========================

func TestUserRegistration(t *testing.T) {
	client := NewTestClient("")

	registerPayload := map[string]interface{}{
		"email":         "testuser@example.com",
		"password":      "secure_password_123",
		"name":          "Test User",
		"role":          "student",
		"dateOfBirth":   "2010-05-15",
	}

	status, body, err := client.Do("POST", "/auth/register", registerPayload)
	if err != nil {
		t.Fatalf("Registration request failed: %v", err)
	}

	if status != http.StatusCreated {
		t.Errorf("Expected status 201, got %d. Response: %s", status, string(body))
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["id"] == nil {
		t.Error("Expected user ID in response")
	}
	if response["token"] == nil {
		t.Error("Expected token in response")
	}
}

func TestUserLogin(t *testing.T) {
	client := NewTestClient("")

	loginPayload := map[string]interface{}{
		"email":    "testuser@example.com",
		"password": "secure_password_123",
	}

	status, body, err := client.Do("POST", "/auth/login", loginPayload)
	if err != nil {
		t.Fatalf("Login request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["token"] == nil {
		t.Error("Expected token in login response")
	}
}

func TestGetUserProfile(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/users/"+testUserID, nil)
	if err != nil {
		t.Fatalf("Get profile request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var profile map[string]interface{}
	json.Unmarshal(body, &profile)

	if profile["id"] != testUserID {
		t.Errorf("Expected user ID %s, got %v", testUserID, profile["id"])
	}
}

func TestUpdateUserProfile(t *testing.T) {
	client := NewTestClient(testToken)

	updatePayload := map[string]interface{}{
		"name":     "Updated Name",
		"timezone": "America/Los_Angeles",
	}

	status, _, err := client.Do("PUT", "/users/"+testUserID, updatePayload)
	if err != nil {
		t.Fatalf("Update profile request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}
}

// ========================
// CONTENT SERVICE TESTS
// ========================

func TestGetSkills(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/skills?grade=3&subject=math", nil)
	if err != nil {
		t.Fatalf("Get skills request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["skills"] == nil {
		t.Error("Expected skills array in response")
	}
}

func TestGetExercises(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/exercises?skillId="+testSkillID, nil)
	if err != nil {
		t.Fatalf("Get exercises request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["exercises"] == nil {
		t.Error("Expected exercises array in response")
	}
}

// ========================
// PROGRESS SERVICE TESTS
// ========================

func TestSubmitExerciseAttempt(t *testing.T) {
	client := NewTestClient(testToken)

	attemptPayload := map[string]interface{}{
		"exerciseId": testExerciseID,
		"skillId":    testSkillID,
		"selectedIndex": 1,
		"timeTaken":  45,
		"timestamp":  time.Now().Format(time.RFC3339),
	}

	status, body, err := client.Do("POST", "/progress/attempts", attemptPayload)
	if err != nil {
		t.Fatalf("Submit attempt request failed: %v", err)
	}

	if status != http.StatusCreated {
		t.Errorf("Expected status 201, got %d. Response: %s", status, string(body))
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["attemptId"] == nil {
		t.Error("Expected attemptId in response")
	}
	if response["isCorrect"] == nil {
		t.Error("Expected isCorrect in response")
	}
}

func TestGetSkillProgress(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/progress/skills/"+testUserID, nil)
	if err != nil {
		t.Fatalf("Get progress request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["skills"] == nil {
		t.Error("Expected skills array in response")
	}
}

// ========================
// TEACHER SERVICE TESTS
// ========================

func TestCreateClassroom(t *testing.T) {
	client := NewTestClient(testToken)

	classroomPayload := map[string]interface{}{
		"name":         "Grade 3 Math",
		"grade":        3,
		"subject":      "math",
		"maxStudents":  30,
		"inviteCode":   "MATH3A",
	}

	status, body, err := client.Do("POST", "/teachers/classrooms", classroomPayload)
	if err != nil {
		t.Fatalf("Create classroom request failed: %v", err)
	}

	if status != http.StatusCreated {
		t.Errorf("Expected status 201, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["id"] == nil {
		t.Error("Expected classroom ID in response")
	}
}

func TestGetClassroomProgress(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/teachers/classrooms/"+testClassroomID+"/progress", nil)
	if err != nil {
		t.Fatalf("Get classroom progress request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["classProgress"] == nil {
		t.Error("Expected classProgress in response")
	}
}

// ========================
// PARENT SERVICE TESTS
// ========================

func TestLinkChild(t *testing.T) {
	client := NewTestClient(testToken)

	linkPayload := map[string]interface{}{
		"childEmail": "student@example.com",
	}

	status, _, err := client.Do("POST", "/parents/link-child", linkPayload)
	if err != nil {
		t.Fatalf("Link child request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}
}

func TestGetChildren(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/parents/children", nil)
	if err != nil {
		t.Fatalf("Get children request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["children"] == nil {
		t.Error("Expected children array in response")
	}
}

// ========================
// NOTIFICATION SERVICE TESTS
// ========================

func TestUpdateNotificationPreferences(t *testing.T) {
	client := NewTestClient(testToken)

	preferencesPayload := map[string]interface{}{
		"emailNotifications": map[string]bool{
			"masteryMilestone": true,
			"alert":            true,
			"dailySummary":     false,
		},
		"emailFrequency": "daily",
	}

	status, _, err := client.Do("POST", "/notifications/preferences", preferencesPayload)
	if err != nil {
		t.Fatalf("Update preferences request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}
}

func TestGetNotifications(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/notifications?limit=20", nil)
	if err != nil {
		t.Fatalf("Get notifications request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["notifications"] == nil {
		t.Error("Expected notifications array in response")
	}
}

// ========================
// ACHIEVEMENT SERVICE TESTS
// ========================

func TestGetAchievements(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/achievements?unlocked=all", nil)
	if err != nil {
		t.Fatalf("Get achievements request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["achievements"] == nil {
		t.Error("Expected achievements array in response")
	}
}

func TestGetUserStats(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/achievements/stats/"+testUserID, nil)
	if err != nil {
		t.Fatalf("Get stats request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var stats map[string]interface{}
	json.Unmarshal(body, &stats)

	if stats["totalPoints"] == nil {
		t.Error("Expected totalPoints in response")
	}
	if stats["level"] == nil {
		t.Error("Expected level in response")
	}
}

// ========================
// INSIGHTS SERVICE TESTS
// ========================

func TestGetAnalytics(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/insights/analytics", nil)
	if err != nil {
		t.Fatalf("Get analytics request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var analytics map[string]interface{}
	json.Unmarshal(body, &analytics)

	if analytics["totalSkills"] == nil {
		t.Error("Expected totalSkills in response")
	}
	if analytics["averageMastery"] == nil {
		t.Error("Expected averageMastery in response")
	}
}

func TestGetRecommendations(t *testing.T) {
	client := NewTestClient(testToken)

	status, body, err := client.Do("GET", "/insights/recommendations", nil)
	if err != nil {
		t.Fatalf("Get recommendations request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	var response map[string]interface{}
	json.Unmarshal(body, &response)

	if response["recommendations"] == nil {
		t.Error("Expected recommendations array in response")
	}
}

// ========================
// PERFORMANCE TESTS
// ========================

func TestConcurrentRequests(t *testing.T) {
	client := NewTestClient(testToken)
	done := make(chan error, 10)

	for i := 0; i < 10; i++ {
		go func() {
			status, _, err := client.Do("GET", "/progress/skills/"+testUserID, nil)
			if err != nil {
				done <- err
			} else if status != http.StatusOK {
				done <- fmt.Errorf("expected 200, got %d", status)
			} else {
				done <- nil
			}
		}()
	}

	for i := 0; i < 10; i++ {
		if err := <-done; err != nil {
			t.Errorf("Concurrent request failed: %v", err)
		}
	}
}

func TestResponseTime(t *testing.T) {
	client := NewTestClient(testToken)

	start := time.Now()
	status, _, err := client.Do("GET", "/progress/skills/"+testUserID, nil)
	elapsed := time.Since(start)

	if err != nil {
		t.Fatalf("Request failed: %v", err)
	}

	if status != http.StatusOK {
		t.Errorf("Expected status 200, got %d", status)
	}

	if elapsed > 1*time.Second {
		t.Logf("Warning: Request took %v (expected < 1s)", elapsed)
	}
}

// ========================
// MAIN TEST RUNNER
// ========================

func TestMain(m *testing.M) {
	fmt.Println("=== PATHFINDER Integration Test Suite ===")
	fmt.Println("Testing all microservices...")
	m.Run()
}
