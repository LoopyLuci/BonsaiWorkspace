import { useQuery } from '@tanstack/react-query'
import axios from 'axios'

export default function TeacherDashboard() {
  const { data: stats } = useQuery({
    queryKey: ['classroom-stats'],
    queryFn: () =>
      axios.get('/api/v1/analytics/classroom/class_1/stats'),
  })

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto">
        <h1 className="text-3xl font-bold mb-8">Class Analytics</h1>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-bold mb-4">Class Performance</h2>
            <div className="space-y-4">
              <div>
                <p className="text-gray-600">Average Mastery</p>
                <div className="w-full bg-gray-200 rounded-full h-2 mt-2">
                  <div
                    className="bg-blue-600 h-2 rounded-full"
                    style={{
                      width: `${stats?.data?.average_mastery * 100 || 0}%`,
                    }}
                  ></div>
                </div>
                <p className="text-sm mt-1">
                  {stats?.data?.average_mastery
                    ? `${Math.round(stats.data.average_mastery * 100)}%`
                    : '0%'
                  }
                </p>
              </div>

              <div>
                <p className="text-gray-600">Completion Rate</p>
                <div className="w-full bg-gray-200 rounded-full h-2 mt-2">
                  <div
                    className="bg-green-600 h-2 rounded-full"
                    style={{
                      width: `${stats?.data?.completion_rate * 100 || 0}%`,
                    }}
                  ></div>
                </div>
                <p className="text-sm mt-1">
                  {stats?.data?.completion_rate
                    ? `${Math.round(stats.data.completion_rate * 100)}%`
                    : '0%'
                  }
                </p>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-bold mb-4">Student Overview</h2>
            <div className="space-y-2">
              <p>
                <span className="text-gray-600">Total Students:</span>{' '}
                <span className="font-bold">{stats?.data?.total_students || 0}</span>
              </p>
              <p>
                <span className="text-gray-600">At Risk:</span>{' '}
                <span className="font-bold text-red-600">
                  {stats?.data?.at_risk_count || 0}
                </span>
              </p>
              <p>
                <span className="text-gray-600">Succeeding:</span>{' '}
                <span className="font-bold text-green-600">
                  {stats?.data?.total_students - (stats?.data?.at_risk_count || 0) || 0}
                </span>
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
