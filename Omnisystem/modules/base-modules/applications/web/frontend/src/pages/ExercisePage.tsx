import { useState } from 'react'
import { useParams } from 'react-router-dom'
import { useQuery, useMutation } from '@tanstack/react-query'
import { exercisesAPI } from '../api'

export default function ExercisePage() {
  const { id } = useParams()
  const [answer, setAnswer] = useState('')
  const [feedback, setFeedback] = useState('')

  const { data: exercise, isLoading } = useQuery({
    queryKey: ['exercise', id],
    queryFn: () => exercisesAPI.get(id || ''),
  })

  const submitMutation = useMutation({
    mutationFn: () =>
      exercisesAPI.submitAttempt({ exercise_id: id, answer }),
    onSuccess: (response) => {
      setFeedback(response.data.feedback)
      setAnswer('')
    },
  })

  if (isLoading) return <div className="p-8">Loading...</div>

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto max-w-2xl">
        <h1 className="text-3xl font-bold mb-8">{exercise?.data?.title}</h1>

        <div className="bg-white rounded-lg shadow p-8">
          <form
            onSubmit={(e) => {
              e.preventDefault()
              submitMutation.mutate()
            }}
            className="space-y-4"
          >
            <div>
              <label className="block text-gray-700 text-sm font-bold mb-2">
                Your Answer
              </label>
              <input
                type="text"
                value={answer}
                onChange={(e) => setAnswer(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:border-blue-500"
                required
              />
            </div>

            <button
              type="submit"
              disabled={submitMutation.isPending}
              className="w-full bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 rounded-lg disabled:opacity-50"
            >
              {submitMutation.isPending ? 'Submitting...' : 'Submit Answer'}
            </button>
          </form>

          {feedback && (
            <div className="mt-6 p-4 bg-green-100 border border-green-400 text-green-700 rounded">
              {feedback}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
