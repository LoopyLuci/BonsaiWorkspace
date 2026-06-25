// PATHFINDER Performance & Load Testing Suite
// k6 load testing configuration

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// ==========================================
// CONFIGURATION
// ==========================================

export const options = {
  stages: [
    { duration: '30s', target: 20 },   // Ramp to 20 users over 30s
    { duration: '1m', target: 20 },    // Sustain 20 users for 1 minute
    { duration: '30s', target: 50 },   // Ramp to 50 users over 30s
    { duration: '2m', target: 50 },    // Sustain 50 users for 2 minutes
    { duration: '30s', target: 0 },    // Scale down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],  // 95th percentile < 500ms, 99th < 1s
    http_req_failed: ['rate<0.1'],                    // Error rate < 10%
    'http_requests': ['rate>100'],                    // At least 100 requests/sec
  },
  ext: {
    loadimpact: {
      projectID: 3520721,
      name: 'PATHFINDER Load Test',
    },
  },
};

// ==========================================
// CUSTOM METRICS
// ==========================================

const apiErrors = new Counter('api_errors');
const apiSuccess = new Counter('api_success');
const loginTime = new Trend('login_time');
const exerciseSubmitTime = new Trend('exercise_submit_time');
const progressFetchTime = new Trend('progress_fetch_time');
const authErrors = new Rate('auth_errors');

// ==========================================
// TEST DATA
// ==========================================

const BASE_URL = 'http://localhost:8001';
const TEST_USERS = [
  { email: 'user1@test.com', password: 'password123' },
  { email: 'user2@test.com', password: 'password123' },
  { email: 'user3@test.com', password: 'password123' },
  { email: 'user4@test.com', password: 'password123' },
  { email: 'user5@test.com', password: 'password123' },
];

const SKILLS = [
  'skill_120', 'skill_121', 'skill_122', 'skill_123', 'skill_124',
  'skill_125', 'skill_126', 'skill_127', 'skill_128', 'skill_129',
];

const EXERCISES = [
  'ex_100', 'ex_101', 'ex_102', 'ex_103', 'ex_104',
  'ex_105', 'ex_106', 'ex_107', 'ex_108', 'ex_109',
];

// ==========================================
// HELPER FUNCTIONS
// ==========================================

function getRandomElement(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}

function sleep_between(min, max) {
  const duration = Math.random() * (max - min) + min;
  sleep(duration / 1000);
}

// ==========================================
// SCENARIO 1: Authentication Flow
// ==========================================

export function scenarioAuthentication() {
  group('Authentication', () => {
    const testUser = TEST_USERS[Math.floor(Math.random() * TEST_USERS.length)];

    // Login
    const loginStart = Date.now();
    const loginRes = http.post(`${BASE_URL}/v1/auth/login`, {
      email: testUser.email,
      password: testUser.password,
    });
    const loginDuration = Date.now() - loginStart;

    check(loginRes, {
      'login status is 200': (r) => r.status === 200,
      'login response has token': (r) => r.json('token') !== null,
      'login time < 500ms': (r) => loginDuration < 500,
    });

    if (loginRes.status === 200) {
      apiSuccess.add(1);
      loginTime.add(loginDuration);
      const token = loginRes.json('token');
      return token;
    } else {
      apiErrors.add(1);
      authErrors.add(1);
      return null;
    }
  });
}

// ==========================================
// SCENARIO 2: Exercise Submission Flow
// ==========================================

export function scenarioExerciseSubmission(token) {
  group('Exercise Submission', () => {
    if (!token) return;

    const skillId = getRandomElement(SKILLS);
    const exerciseId = getRandomElement(EXERCISES);

    const headers = {
      'Content-Type': 'application/json',
      'X-User-ID': 'test_user',
      'Authorization': `Bearer ${token}`,
    };

    // Get exercise details
    const getExerciseRes = http.get(
      `${BASE_URL}/v1/exercises/${exerciseId}`,
      { headers }
    );

    check(getExerciseRes, {
      'get exercise status is 200': (r) => r.status === 200,
      'get exercise has options': (r) => r.json('options') !== null,
    });

    sleep_between(2, 5); // Simulate student thinking time

    // Submit attempt
    const submitStart = Date.now();
    const submitRes = http.post(
      `${BASE_URL}/v1/progress/attempts`,
      JSON.stringify({
        exerciseId: exerciseId,
        skillId: skillId,
        selectedIndex: Math.floor(Math.random() * 4),
        timeTaken: Math.floor(Math.random() * 120) + 20,
        timestamp: new Date().toISOString(),
      }),
      { headers }
    );
    const submitDuration = Date.now() - submitStart;

    check(submitRes, {
      'submit attempt status is 201': (r) => r.status === 201,
      'submit attempt has attemptId': (r) => r.json('attemptId') !== null,
      'submit attempt time < 1000ms': (r) => submitDuration < 1000,
    });

    if (submitRes.status === 201) {
      apiSuccess.add(1);
      exerciseSubmitTime.add(submitDuration);
    } else {
      apiErrors.add(1);
    }
  });
}

// ==========================================
// SCENARIO 3: Progress Tracking
// ==========================================

export function scenarioProgressTracking(token) {
  group('Progress Tracking', () => {
    if (!token) return;

    const headers = {
      'Content-Type': 'application/json',
      'X-User-ID': 'test_user',
      'Authorization': `Bearer ${token}`,
    };

    // Get skill progress
    const progressStart = Date.now();
    const progressRes = http.get(
      `${BASE_URL}/v1/progress/skills/test_user`,
      { headers }
    );
    const progressDuration = Date.now() - progressStart;

    check(progressRes, {
      'progress fetch status is 200': (r) => r.status === 200,
      'progress has skills array': (r) => r.json('skills') !== null,
      'progress fetch time < 300ms': (r) => progressDuration < 300,
    });

    if (progressRes.status === 200) {
      apiSuccess.add(1);
      progressFetchTime.add(progressDuration);
    } else {
      apiErrors.add(1);
    }

    sleep_between(1, 2);

    // Get analytics
    const analyticsRes = http.get(
      `${BASE_URL}/v1/insights/analytics`,
      { headers }
    );

    check(analyticsRes, {
      'analytics fetch status is 200': (r) => r.status === 200,
      'analytics has totalSkills': (r) => r.json('totalSkills') !== null,
    });

    if (analyticsRes.status !== 200) {
      apiErrors.add(1);
    }
  });
}

// ==========================================
// SCENARIO 4: Content Browsing
// ==========================================

export function scenarioContentBrowsing(token) {
  group('Content Browsing', () => {
    if (!token) return;

    const headers = {
      'Content-Type': 'application/json',
      'X-User-ID': 'test_user',
      'Authorization': `Bearer ${token}`,
    };

    // Get skills
    const skillsRes = http.get(
      `${BASE_URL}/v1/skills?grade=3&subject=math&limit=20`,
      { headers }
    );

    check(skillsRes, {
      'skills fetch status is 200': (r) => r.status === 200,
      'skills has array': (r) => r.json('skills') !== null,
    });

    sleep(1);

    // Get exercises for a skill
    const skillId = getRandomElement(SKILLS);
    const exercisesRes = http.get(
      `${BASE_URL}/v1/exercises?skillId=${skillId}&limit=10`,
      { headers }
    );

    check(exercisesRes, {
      'exercises fetch status is 200': (r) => r.status === 200,
      'exercises has array': (r) => r.json('exercises') !== null,
    });
  });
}

// ==========================================
// SCENARIO 5: Gamification Access
// ==========================================

export function scenarioGamificationAccess(token) {
  group('Gamification', () => {
    if (!token) return;

    const headers = {
      'Content-Type': 'application/json',
      'X-User-ID': 'test_user',
      'Authorization': `Bearer ${token}`,
    };

    // Get achievements
    const achievementsRes = http.get(
      `${BASE_URL}/v1/achievements?unlocked=all`,
      { headers }
    );

    check(achievementsRes, {
      'achievements fetch status is 200': (r) => r.status === 200,
      'achievements has array': (r) => r.json('achievements') !== null,
    });

    sleep(1);

    // Get user stats
    const statsRes = http.get(
      `${BASE_URL}/v1/achievements/stats/test_user`,
      { headers }
    );

    check(statsRes, {
      'stats fetch status is 200': (r) => r.status === 200,
      'stats has totalPoints': (r) => r.json('totalPoints') !== null,
    });

    sleep(1);

    // Get leaderboard
    const leaderboardRes = http.get(
      `${BASE_URL}/v1/leaderboard?timeRange=month&limit=100`,
      { headers }
    );

    check(leaderboardRes, {
      'leaderboard fetch status is 200': (r) => r.status === 200,
      'leaderboard has array': (r) => r.json('leaderboard') !== null,
    });
  });
}

// ==========================================
// SCENARIO 6: Notifications
// ==========================================

export function scenarioNotifications(token) {
  group('Notifications', () => {
    if (!token) return;

    const headers = {
      'Content-Type': 'application/json',
      'X-User-ID': 'test_user',
      'Authorization': `Bearer ${token}`,
    };

    // Get notifications
    const notificationsRes = http.get(
      `${BASE_URL}/v1/notifications?limit=20`,
      { headers }
    );

    check(notificationsRes, {
      'notifications fetch status is 200': (r) => r.status === 200,
      'notifications has array': (r) => r.json('notifications') !== null,
    });

    sleep(1);

    // Get preferences
    const preferencesRes = http.get(
      `${BASE_URL}/v1/notifications/preferences`,
      { headers }
    );

    check(preferencesRes, {
      'preferences fetch status is 200': (r) => r.status === 200,
      'preferences has emailFrequency': (r) => r.json('emailFrequency') !== null,
    });
  });
}

// ==========================================
// MAIN LOAD TEST
// ==========================================

export default function () {
  const token = scenarioAuthentication();

  sleep_between(1, 3);

  scenarioExerciseSubmission(token);
  sleep_between(2, 4);

  scenarioProgressTracking(token);
  sleep_between(1, 2);

  scenarioContentBrowsing(token);
  sleep_between(1, 2);

  scenarioGamificationAccess(token);
  sleep_between(1, 2);

  scenarioNotifications(token);

  // Cool-down between iterations
  sleep_between(3, 5);
}

// ==========================================
// SPIKE TEST (Optional)
// ==========================================

export function spikeTest() {
  const spikeOptions = {
    stages: [
      { duration: '1m', target: 10 },    // Warm up
      { duration: '10s', target: 100 },  // Spike to 100 users
      { duration: '1m', target: 100 },   // Sustain spike
      { duration: '10s', target: 10 },   // Drop back down
      { duration: '30s', target: 0 },    // Cool down
    ],
  };
  // Run default function with spike options
}

// ==========================================
// SOAK TEST (Optional - long duration)
// ==========================================

export function soakTest() {
  const soakOptions = {
    stages: [
      { duration: '5m', target: 30 },     // Ramp to 30 users
      { duration: '8h', target: 30 },     // Soak for 8 hours
      { duration: '5m', target: 0 },      // Cool down
    ],
  };
  // Run default function with soak options
}

// ==========================================
// STRESS TEST (Optional - push limits)
// ==========================================

export function stressTest() {
  const stressOptions = {
    stages: [
      { duration: '2m', target: 50 },
      { duration: '2m', target: 100 },
      { duration: '2m', target: 200 },
      { duration: '2m', target: 300 },
      { duration: '2m', target: 0 },
    ],
  };
  // Run default function with stress options
}

// ==========================================
// REPORTING
// ==========================================

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'summary.json': JSON.stringify(data),
    'summary.html': htmlReport(data),
  };
}

function textSummary(data, options) {
  const summary = [
    '=== PATHFINDER Load Test Results ===\n',
    `Total Requests: ${data.metrics.http_reqs?.value || 0}`,
    `Failed Requests: ${data.metrics.http_req_failed?.value || 0}`,
    `Success Rate: ${((1 - (data.metrics.http_req_failed?.value || 0)) * 100).toFixed(2)}%`,
    `Avg Response Time: ${(data.metrics.http_req_duration?.value || 0).toFixed(2)}ms`,
    `P95 Response Time: ${((data.metrics.http_req_duration?.values?.['p(95)']) || 0).toFixed(2)}ms`,
    `P99 Response Time: ${((data.metrics.http_req_duration?.values?.['p(99)']) || 0).toFixed(2)}ms`,
  ];
  return summary.join('\n');
}

function htmlReport(data) {
  return `
    <html>
      <head><title>PATHFINDER Load Test Report</title></head>
      <body>
        <h1>PATHFINDER Performance Test Results</h1>
        <p>Report generated at: ${new Date().toISOString()}</p>
        <pre>${JSON.stringify(data, null, 2)}</pre>
      </body>
    </html>
  `;
}
