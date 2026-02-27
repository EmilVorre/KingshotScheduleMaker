export default function InfoPage() {
  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <header className="text-center mb-12">
        <h1 className="text-5xl font-bold text-blue-400 mb-4">
          <i className="fas fa-info-circle mr-3"></i>How to Use
        </h1>
        <p className="text-xl text-gray-400">Guide for using the Schedule Maker and in-game requirements</p>
      </header>

      <main className="space-y-8">
        {/* For Admins */}
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-2xl font-bold text-blue-400 mb-6">
            <i className="fas fa-tools mr-2"></i>For Admins: How to Use the Tools
          </h2>
          <div className="prose prose-invert text-gray-300 space-y-6">
            <div>
              <h3 className="text-xl font-semibold text-white">1. In-Game: Setting Up Appointments</h3>
              <p>When it&apos;s time to run the schedule in-game, follow these steps:</p>

              <h4 className="text-lg font-semibold text-gray-200 mt-4">1 hour before reset</h4>
              <ul className="list-disc pl-6 space-y-1">
                <li>Turn <strong>auto accept</strong> off on the appointment</li>
                <li>Clear the queue</li>
                <li>The appointment is now ready to be used</li>
              </ul>

              <h4 className="text-lg font-semibold text-gray-200 mt-4">15 minutes before reset (Slot 1 / 00:00)</h4>
              <ul className="list-disc pl-6 space-y-1">
                <li>Add the <strong>first person</strong> from the schedule to the position</li>
                <li>The 15 minutes <em>before</em> reset are for positioning only – do not use the slot during this time</li>
                <li>The 15 minutes <em>after</em> reset are when their appointment time actually starts</li>
                <li><strong>Why 15 min instead of 30?</strong> Slot 1 needs the 15 min before reset to get the first person into place. This will ensure that we have 49 slots too fill instead of 48. All other slots are 30 minutes because the next person can be queued while the previous one is using the appointment.</li>
              </ul>

              <h4 className="text-lg font-semibold text-gray-200 mt-4">After reset – rest of the schedule</h4>
              <ul className="list-disc pl-6 space-y-1">
                <li>Add the remaining players either <strong>all at once</strong> or <strong>gradually over time</strong></li>
                <li>Adding gradually lets people use other appointments instead of waiting in the queue (personal preference)</li>
              </ul>

              <h4 className="text-lg font-semibold text-gray-200 mt-4">Back-to-back days (e.g. Construction → Research)</h4>
              <p>When two days use the same appointment and run back-to-back, there is a <strong>crossover slot</strong> between Day 1 and Day 2:</p>
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>What it is:</strong> Each day runs from 15 min before reset to 15 min after reset the next day. When Day 2 follows Day 1, the end of Day 1 (15 min after Day 1&apos;s reset) overlaps with the start of Day 2 (15 min before Day 2&apos;s reset). One person occupies this overlap – they get 15 min at the end of Day 1 and 15 min at the start of Day 2.</li>
                <li>The program only assigns someone here if they applied for <strong>both</strong> days</li>
                <li><strong>What to do:</strong> When the last person from Day 1 has been added, extend the queue to include the Day 2 schedule, starting with <strong>slot 2</strong> (slot 1 is the crossover person, already in the queue from Day 1)</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">2. Create Account</h3>
              <p>From the home page, click <strong>Create Account</strong>. Enter:</p>
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>Account name</strong> – Used in your schedule URL (e.g. <code className="bg-gray-700 px-1 rounded">/accountname/1</code>)</li>
                <li><strong>Server number</strong> – Your Kingshot server number</li>
                <li><strong>In-game name</strong> – Your character name</li>
                <li><strong>Password</strong> – To access your dashboard</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">3. Create Form</h3>
              <p>In the dashboard, go to the <strong>Create Form</strong> tab. Configure:</p>
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>Alliances</strong> – Add your alliance names (one per line). Players will select from this list. Include &quot;Non of the above&quot; if you want users to type a custom alliance.</li>
                <li><strong>Current Age of Your Kingdom</strong> – Choose based on your server state:</li>
                <ul className="ml-6 list-disc space-y-1">
                  <li><strong>Pre-truegold</strong> – No truegold fields shown</li>
                  <li><strong>Truegold unlocked</strong> – Construction truegold only</li>
                  <li><strong>War academy unlocked</strong> – Construction truegold + Research truegold dust</li>
                  <li><strong>Tempered truegold unlocked</strong> – Construction truegold + tempered truegold + Research truegold dust</li>
                </ul>
                <li><strong>Time slots</strong> – The schedule is fixed for each day: from <strong>15 minutes before reset</strong> to <strong>15 minutes after reset the following day</strong> (24 hours 30 minutes total). All times are in <strong>UTC</strong>.</li>
                <li><strong>Intro text</strong> – Custom message shown at the top of the form.</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">4. Share Form Link</h3>
              <p>From the <strong>Current Form</strong> tab:</p>
              <ul className="list-disc pl-6 space-y-1">
                <li>Copy the form link (e.g. <code className="bg-gray-700 px-1 rounded">/form/ABC123</code>)</li>
                <li>Share it with your players via Discord, in-game mail, etc.</li>
                <li>You can download the submissions CSV from this tab</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">5. Generate Schedule</h3>
              <p>From the <strong>Generate Schedule</strong> tab:</p>
              <ul className="list-disc pl-6 space-y-1">
                <li>Add <strong>predetermined slots</strong> to assign a specific time slot to a specific player (e.g. give Player X the 23:45 slot on Construction day)</li>
                <li>Click <strong>Generate Schedule</strong> – the scheduler prioritizes by score</li>
                <li><strong>Construction day</strong> – Score = (truegold × 2000) + (tempered truegold × 30000) + (speedups × 30). Highest score gets first pick; slots can be &quot;stolen&quot; by higher-scoring players.</li>
                <li><strong>Research day</strong> – Score = (truegold dust × 1000) + (speedups × 30). The player in the <em>last</em> Construction slot gets priority for slot 1 (handoff continuity).</li>
                <li><strong>Troops day</strong> – Prioritized by speedups</li>
                <li>After generation, go to the <strong>Schedules</strong> page to view and edit slots manually</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">6. CSV Upload (Alternative)</h3>
              <p>From the <strong>CSV Operations</strong> tab (or <strong>Upload CSV</strong> in the admin panel):</p>
              <ul className="list-disc pl-6 space-y-1">
                <li>If you collect submissions via Google Forms or another tool, export to CSV</li>
                <li>Upload the CSV – it must match the expected column format</li>
                <li>Then generate the schedule as usual</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">7. Statistics</h3>
              <p>The <strong>Statistics</strong> tab shows submission counts by alliance and time slot popularity, useful for planning.</p>
            </div>
          </div>
        </div>

        {/* Priority / Scoring */}
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-2xl font-bold text-purple-400 mb-6">
            <i className="fas fa-calculator mr-2"></i>How Priority Is Calculated
          </h2>
          <div className="prose prose-invert text-gray-300 space-y-4">
            <p>Higher scores get higher priority when assigning slots. The scheduler uses the following formulas:</p>
            <div className="bg-gray-700/50 rounded-lg p-4 border border-gray-600 font-mono text-sm space-y-2">
              <p><strong className="text-orange-400">Construction:</strong> (truegold × 2000) + (tempered truegold × 30000) + (speedups × 30)</p>
              <p><strong className="text-blue-400">Research:</strong> (truegold dust × 1000) + (speedups × 30)</p>
              <p><strong className="text-green-400">Troops:</strong> speedups only</p>
            </div>
            <p className="text-sm text-gray-400">Speedups are in hours. Add general speedups + day-specific speedups (e.g. construction speedups = general + construction).</p>
          </div>
        </div>

        {/* For Players: In-Game Requirements */}
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-2xl font-bold text-green-400 mb-6">
            <i className="fas fa-gamepad mr-2"></i>For Players: In-Game Requirements
          </h2>
          <div className="prose prose-invert text-gray-300 space-y-6">
            <p className="text-lg">Before filling out the form, gather the following from your Kingshot game:</p>

            <div>
              <h3 className="text-xl font-semibold text-white">Player ID</h3>
              <ul className="list-disc pl-6 space-y-1">
                <li>Your numeric player ID – found in your profile</li>
                <li>Must be digits only (no letters)</li>
                <li>Use the <strong>Confirm</strong> button – it will look up and fill in your character name automatically</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">Construction Day</h3>
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>Speedups</strong> – Total hours of general speedups + construction speedups you plan to use</li>
                <li><strong>Truegold</strong> – Amount of truegold you plan to spend (if applicable)</li>
                <li><strong>Tempered Truegold</strong> – Amount of tempered truegold (if applicable)</li>
                <li>Select at least <strong>5 time slots</strong> when you are available</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">Research Day</h3>
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>Speedups</strong> – Total hours of general + research speedups you plan to use</li>
                <li><strong>Truegold Dust</strong> – Amount of truegold dust you plan to spend (if applicable)</li>
                <li>Select at least <strong>5 time slots</strong> when you are available</li>
              </ul>
            </div>

            <div>
              <h3 className="text-xl font-semibold text-white">Troops Training Day</h3>
              <ul className="list-disc pl-6 space-y-1">
                <li><strong>Speedups</strong> – Total hours of general + troops training speedups you plan to use</li>
                <li>Select at least <strong>5 time slots</strong> when you are available</li>
              </ul>
            </div>
          </div>
        </div>

        {/* Tips */}
        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <h2 className="text-2xl font-bold text-amber-400 mb-6">
            <i className="fas fa-lightbulb mr-2"></i>Tips &amp; Best Practices
          </h2>
          <div className="space-y-4">
            <div className="flex gap-4 p-4 bg-gray-700/50 rounded-lg border border-gray-600">
              <i className="fas fa-check-circle text-green-400 text-2xl flex-shrink-0 mt-1"></i>
              <div>
                <strong className="text-white">Submit early</strong> – The best way to avoid forgetting to submit or missing the deadline.
              </div>
            </div>
            <div className="flex gap-4 p-4 bg-gray-700/50 rounded-lg border border-gray-600">
              <i className="fas fa-check-circle text-green-400 text-2xl flex-shrink-0 mt-1"></i>
              <div>
                <strong className="text-white">Choose multiple times</strong> – You must select at least 5 slots per day. Selecting more increases your chances of getting a slot.
              </div>
            </div>
            <div className="flex gap-4 p-4 bg-gray-700/50 rounded-lg border border-gray-600">
              <i className="fas fa-check-circle text-green-400 text-2xl flex-shrink-0 mt-1"></i>
              <div>
                <strong className="text-white">Construction → Research handoff</strong> – The player in the last Construction slot gets priority for slot 1 on Research day (for continuity).
              </div>
            </div>
            <div className="flex gap-4 p-4 bg-gray-700/50 rounded-lg border border-gray-600">
              <i className="fas fa-check-circle text-green-400 text-2xl flex-shrink-0 mt-1"></i>
              <div>
                <strong className="text-white">Predetermined slots</strong> – Admins can assign a specific time slot to a specific player (e.g. give Player X the 23:45 slot). These slots are reserved before generation.
              </div>
            </div>
          </div>
        </div>
      </main>

    </div>
  )
}
