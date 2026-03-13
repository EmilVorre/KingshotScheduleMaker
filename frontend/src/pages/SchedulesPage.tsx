import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api, type Schedule } from '../api/client'

const DAYS = [
  { key: 'construction', name: 'Construction Day', icon: 'fa-hammer', btnClass: 'bg-orange-600 hover:bg-orange-700', ringClass: 'ring-4 ring-orange-400' },
  { key: 'research', name: 'Research Day', icon: 'fa-flask', btnClass: 'bg-blue-600 hover:bg-blue-700', ringClass: 'ring-4 ring-blue-400' },
  { key: 'troops', name: 'Troops Training Day', icon: 'fa-users', btnClass: 'bg-green-600 hover:bg-green-700', ringClass: 'ring-4 ring-green-400' },
] as const

export default function SchedulesPage() {
  const { accountName, formId } = useParams<{ accountName: string; formId: string }>()
  const [currentDay, setCurrentDay] = useState<(typeof DAYS)[number]['key']>('construction')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [schedule, setSchedule] = useState<Schedule | null>(null)

  useEffect(() => {
    if (!accountName || !formId) return
    setLoading(true)
    setError(null)
    api.getScheduleByFormCode(accountName, formId, currentDay).then(({ ok, data, error: err }) => {
      if (ok && data) setSchedule(data)
      else setError(err || 'Failed to load schedule')
      setLoading(false)
    })
  }, [accountName, formId, currentDay])

  if (!accountName || !formId) {
    return (
      <div className="container mx-auto px-4 py-8">
        <p className="text-red-400">Invalid URL</p>
        <Link to="/" className="text-blue-400 mt-4 inline-block">← Home</Link>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-5xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-calendar-check mr-3"></i>Schedules
        </h1>
        <p className="text-gray-400">View Appointment Schedules</p>
      </header>

      <div className="bg-gray-800 rounded-lg shadow-xl p-8 mb-6 border border-gray-700">
        <div className="flex justify-center gap-4 flex-wrap">
          {DAYS.map((d) => (
            <button
              key={d.key}
              onClick={() => setCurrentDay(d.key)}
              className={`px-6 py-3 rounded-lg font-semibold transition-all shadow-lg text-white ${d.btnClass} ${currentDay === d.key ? d.ringClass : ''}`}
            >
              <i className={`fas ${d.icon} mr-2`}></i>{d.name}
            </button>
          ))}
        </div>
      </div>

      {loading && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
          <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
          <p className="text-xl text-gray-400">Loading schedule...</p>
        </div>
      )}
      {error && (
        <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
          <i className="fas fa-exclamation-circle mr-2"></i>{error}
        </div>
      )}
      {!loading && !error && schedule?.appointments && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-3xl font-bold text-white mb-6 text-center">{schedule.day_name}</h2>
          <div className="border-2 border-gray-700 rounded-lg overflow-hidden">
            {schedule.appointments.map((slot) => (
              <div
                key={slot.time}
                className={`flex items-center p-3 border-b border-gray-700 hover:bg-gray-700/50 transition-colors ${slot.is_empty ? 'opacity-60' : ''}`}
              >
                <span className={`w-24 font-bold ${slot.is_empty ? 'text-gray-500' : 'text-blue-400'}`}>{slot.time}</span>
                <span className={slot.is_empty ? 'text-gray-500 italic' : 'text-gray-200 font-medium'}>
                  {slot.is_empty ? '[EMPTY]' : slot.player}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
      {!loading && !error && !schedule?.appointments?.length && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
          <i className="fas fa-calendar-times text-4xl text-gray-500 mb-4"></i>
          <p className="text-xl text-gray-400">No schedule data available. Generate a schedule first.</p>
        </div>
      )}

    </div>
  )
}
