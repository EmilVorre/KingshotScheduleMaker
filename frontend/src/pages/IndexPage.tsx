import { useAuth } from '../context/AuthContext'
import { ENABLE_GOOGLE_LOGIN, IS_DEV } from '../config'

export default function IndexPage() {
  const { isValid } = useAuth()

  return (
    <div className="container mx-auto px-4 py-8 max-w-6xl">
      <header className="text-center mb-12">
        <h1 className="text-5xl font-bold text-blue-400 mb-4">
          <i className="fas fa-calendar-alt mr-3"></i>Schedule Maker
        </h1>
        <p className="text-xl text-gray-400">Create and manage appointment schedules for multiple servers</p>
      </header>

      <main className="space-y-8">
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <div className="text-center mb-8">
            <h2 className="text-3xl font-bold text-white mb-3">Welcome</h2>
            <p className="text-gray-400 text-lg">
              {isValid ? 'You are signed in. Use the sidebar to access your dashboard.' : 'Sign in to access your schedule'}
            </p>
          </div>

          {!isValid && (
            <div className="flex flex-col gap-4 max-w-md mx-auto">
              <p className="text-sm font-medium text-gray-400 text-center">Sign in with</p>
              <div className="flex flex-col gap-3">
                {IS_DEV && (
                  <div className="flex gap-2">
                    <a
                      href="/api/dev-login"
                      className="flex-1 flex items-center justify-center gap-2 px-6 py-3 bg-amber-600 hover:bg-amber-500 text-white rounded-lg font-semibold text-sm border border-amber-500"
                    >
                      <i className="fas fa-bolt"></i>
                      Dev: Admin (devtest)
                    </a>
                    <a
                      href="/api/dev-login-user"
                      className="flex-1 flex items-center justify-center gap-2 px-6 py-3 bg-amber-700 hover:bg-amber-600 text-white rounded-lg font-semibold text-sm border border-amber-600"
                    >
                      <i className="fas fa-user"></i>
                      Dev: User (devuser)
                    </a>
                  </div>
                )}
                <div className="flex gap-3">
                <a
                  href="/api/auth/discord"
                  className="flex-1 flex items-center justify-center gap-2 px-6 py-4 bg-[#5865F2] hover:bg-[#4752C4] text-white rounded-lg font-semibold text-lg transition-all"
                >
                  <i className="fab fa-discord text-2xl"></i>
                  Discord
                </a>
                {ENABLE_GOOGLE_LOGIN && (
                  <a
                    href="/api/auth/google"
                    className="flex-1 flex items-center justify-center gap-2 px-6 py-4 bg-white hover:bg-gray-100 text-gray-800 rounded-lg font-semibold text-lg transition-all border border-gray-300"
                  >
                    <i className="fab fa-google text-2xl text-red-500"></i>
                    Google
                  </a>
                )}
                </div>
              </div>
              <p className="text-center text-gray-500 text-sm">New? Sign in to create an account</p>
            </div>
          )}
        </div>

        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-2xl font-bold text-white mb-6">How It Works</h2>
          <div className="grid md:grid-cols-4 gap-6">
            <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
              <div className="text-3xl mb-4 text-blue-400">
                <i className="fas fa-user-plus"></i>
              </div>
              <h3 className="text-xl font-bold text-white mb-3">1. Create Account</h3>
              <p className="text-gray-400">Create an account with your account name, server number under profile</p>
            </div>
            <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
              <div className="text-3xl mb-4 text-purple-400">
                <i className="fas fa-file-alt"></i>
              </div>
              <h3 className="text-xl font-bold text-white mb-3">2. Create Form</h3>
              <p className="text-gray-400">Create a form with your alliances and kingdom age</p>
            </div>
            <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
              <div className="text-3xl mb-4 text-green-400">
                <i className="fas fa-share-alt"></i>
              </div>
              <h3 className="text-xl font-bold text-white mb-3">3. Share Form</h3>
              <p className="text-gray-400">Share the form link with players - they fill it out with their preferences</p>
            </div>
            <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
              <div className="text-3xl mb-4 text-orange-400">
                <i className="fas fa-calendar-alt"></i>
              </div>
              <h3 className="text-xl font-bold text-white mb-3">4. Generate Schedule</h3>
              <p className="text-gray-400">Generate optimized schedules from form submissions and edit as needed</p>
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
