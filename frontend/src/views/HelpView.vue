<script setup>
import YesNo from '@/components/ShouldPing.vue';
</script>

<template>
    <div class="container">
        <h1>Help</h1>
        <p>
            This tracker is designed specifically for assisting the organizers
            and players of async multiworld sessions. It helps organize the
            slots in the multiworld, determine where progress can be made, and
            communicate with other players. (Having said that, you can certainly
            use it for single-player multiworld games!)
        </p>
        <p>
            Credit to RadzPrower for designing the Google spreadsheet upon which
            this tracker was based. I designed this tracker in part to overcome
            some of the limitations of spreadsheets in general, and Google
            Sheets in particular.
        </p>
        <p>
            The very first thing you should do is either sign in with Discord
            using the button in the top right, or set your discord username in
            the settings. You must do either of these things before you can
            claim any slots.
        </p>
        <p>
            If you do not sign in to Discord, your claims will be annotated with
            an unauthenticated icon (<i title="Unauthenticated" class="bg-transparent text-warning bi-unlock-fill"></i>) to
            help distinguish authenticated vs unauthenticated claims.
        </p>
        <h2>Tracker Settings</h2>
        <p>
            Clicking the gear button by the tracker title will open up a
            settings pane.  These settings apply to the whole tracker.
        </p>
        <ul>
            <li>
                <b>Organizer</b>: The organizer of this async.  If the tracker
                is unclaimed, clicking the "claim" button will make yourself the
                organizer.  If you are the organizer, you can click "disclaim"
                to disclaim it.  If someone else is the organizer, you cannot
                claim it.
            </li>
            <li>
                <b>Lock settings</b>: If toggled on, only the organizer can
                change the tracker settings.  This setting can only be toggled
                by the organizer.
            </li>
            <li>
                <b>Title</b>: The title displayed at the top of the tracker and
                in the browser tab and/or title bar.
            </li>
            <li>
                <b>Ping policy</b>: Sets the ping policy for the tracker.  If
                set to "none" then participants can set their own ping
                preference per slot.  Otherwise, the effective ping preference
                for each slot is forced to be the same as the tracker's ping
                policy.  See the <a href="#pinging">pinging</a> section for a
                description of each policy.  "Custom" specifies that the async
                has some other ping policy, which should be described in the
                tracker description (once that field is added).
            </li>
        </ul>
        <h2>Tracker Columns</h2>
        <p>
            When you open a tracker, you will be presented with a large table.
            Each row in this table corresponds to one of the slots in the
            multiworld. Some columns can be sorted by clicking their header
            laber, and/or filtered by clicking the filter button in the header.
            Here is a list of the columns of this table and what they represent:
        </p>
        <ul>
            <li>
                <p>
                    <b>Name</b>: This is the name of the slot on Archipelago. If
                    you alias your slot, it will be reflected here.
                </p>
                <p>
                    You can click a slot's name to open a new tab/window
                    containing the AP slot-specific tracker. This can help you
                    quickly determine if a slot has available progression
                    without needing to open the game or the AP text client.
                </p>
            </li>
            <li>
                <b>Ping</b>: Indicates under what circumstances the owner of
                this slot wants to be pinged on Discord regarding this slot. See
                the "pinging" section below for more information about what this
                means and when it is appropriate to ping someone.
            </li>
            <li>
                <p>
                    <b>Availability</b>: Indicates whether the slot is available
                    to claim or play. See the "availability" section below for
                    details.
                </p>
                <p>
                    If this is not your slot, a "claim" button will appear,
                    which can be used to claim the slot for yourself. If this is
                    your slot, a "disclaim" button will appear instead, allowing
                    you to release your claim on the slot. It goes without
                    saying that you should not claim a slot owned by another
                    player without consulting them or the organizer of the
                    multiworld first. (An additional popup confirmation will
                    appear when you attempt to claim someone else's slot to help
                    prevent accidents.)
                </p>
            </li>
            <li>
                <b>Owner</b>: The Discord username of the player who has claimed
                this slot.
            </li>
            <li>
                <b>Game</b>: The game being played in this slot.
            </li>
            <li>
                <b>Status</b>: The status of the game in this slot. This is
                broken down into progression and completion statuses, which are
                each documented in their own sections below.
            </li>
            <li>
                <p>
                    <b>Last Activity</b>: Shows the fractional number of days
                    elapsed since the last activity occurred in this slot. This
                    will be automatically updated when a check is sent out from
                    this slot. If the slot is BK and no progress can be made,
                    the "Still BK" button can be used to reset this counter to
                    signal that you are paying attention to your slot, but you
                    cannot currently progress it.
                </p>
                <p>
                    The value of this column will be
                    <span class="text-success">green</span> for values under 1,
                    <span class="text-warning">yellow</span> for values between
                    1 and 2, and <span class="text-danger">red</span> for values
                    greater than 2. Note that individual multiworld games may
                    have different activity requirements. In the future, the
                    thresholds for the colors in this column may be customizable
                    per tracker.
                </p>
            </li>
            <li>
                <b>Checks</b>: Shows the number of completed and total checks in
                this slot.
            </li>
            <li>
                <p>
                    <b>Hints</b>: Shows the number of unfound hints where this
                    slot is the "finder." If there are any hints (or notes) for
                    this slot, this button will be
                    <span class="text-info">blue</span>. At 5 unfound hints, it
                    will become <span class="text-warning">yellow</span>, and at
                    10, <span class="text-danger">red</span>. Clicking this
                    button will expand the slot's hints and notes pane. See the
                    "hints and notes" section below for more information.
                </p>
                <p>
                    If there are notes for a slot, there will also be an
                    asterisk (*) displayed after the hint count.
                </p>
            </li>
        </ul>
        <h2>Updating the Tracker</h2>
        <p>
            Every time the tracker is loaded or refreshed by clicking the
            "refresh" button, the tracker web service may request an update from
            the Archipelago tracker. This will happen only if it's been more
            than one minute since the Archipelago tracker was last fetched. This
            means that the "checks" column and related summary data at the
            bottom of the sheet may not be updated immediately. If you sent out
            some checks, they may not be reflected on the next refresh.
        </p>
        <p>
            Request merging is performed, so if a bunch of people press the
            refresh button simultaneously, the web service will still only send
            one request to the Archipelago tracker server.
        </p>
        <p>
            If the Archipelago tracker is not fetched, "refresh" will still
            update information managed by this web application -- slot statuses,
            last activity, notes, owner, ping settings, etc. will all be
            updated.
        </p>
        <p>
            You can see the time the Archipelago tracker information was last
            pulled at the bottom of the tracker.
        </p>
        <h2>Availability</h2>
        <p>
            This column communicates whether the slot is available to claim or
            play. The following options are available:
        </p>
        <ul>
            <li>
                <span class="fw-bold text-muted">Unknown</span>: It is not known
                whether the slot is available to play. This is the default
                because different asyncs will start with different slot
                ownership.
            </li>
            <li>
                <span class="fw-bold text-success">Open</span>: This slot is
                open to be claimed. Different asyncs may have different policies
                regarding who may claim slots. For example, some asyncs may have
                a maximum number of slots per player, a maximum number of
                <i>incomplete</i> slots per player, or some other policy. If in
                doubt, contact the organizer before claiming an open slot.
            </li>
            <li>
                <span class="fw-bold text-light">Claimed</span>: This slot has
                been claimed by another user. You should not play on this slot
                or claim it without consulting the owner and/or the organizer.
            </li>
            <li>
                <span class="fw-bold text-info">Public</span>: This slot is
                available for anyone to play without needing to claim it. The
                slot may still have a primary owner.
            </li>
        </ul>
        <p>
            Availability is managed semi-automatically:
        </p>
        <ul>
            <li>
                If a slot marked <span class="fw-bold text-muted">unknown</span> or
                <span class="fw-bold text-success">open</span> is claimed, its
                availability will automatically change to
                <span class="fw-bold text-light">claimed</span>.
            </li>
            <li>
                If a slot marked <span class="fw-bold text-light">claimed</span>
                is disclaimed, its availability will automatically change to
                <span class="fw-bold text-success">open</span>.
            </li>
        </ul>
        <h2>Status</h2>
        <p>
            The status column is used to communicate what is happening with a
            particular slot. This is broken down into progression status and
            completion status.
        </p>
        <p>
            Progression status communicates whether progress can be made on the
            slot.
        </p>
        <ul>
            <li>
                <span class="fw-bold text-muted">Unknown</span>: It is not known
                whether progression is possible. This is the default state. It
                may also be manually selected, for example if someone needs to
                disclaim a slot and doesn't remember if there is available
                progression.
            </li>
            <li>
                <span class="fw-bold text-light">Unblocked</span>: This slot has
                available in-logic progression, or out-of-logic progression that
                the player is not currently pursuing.
            </li>
            <li>
                <span class="fw-bold text-danger">BK</span>: This slot has no
                available in-logic progression, and is waiting on items from
                other slots.
            </li>
            <li>
                <p>
                    <span class="fw-bold text-warning">Soft BK</span>: This slot
                    has remaining in-logic progression that is very difficult or
                    tedious (e.g. farming money), or has out-of-logic
                    progression that the player knows how to obtain but has
                    chosen not to pursue for the moment.
                </p>
                <p>
                    The player is able to obtain those checks if necessary but
                    would prefer to wait until they are sent items that would
                    reduce the difficulty or tedium of obtaining the remaining
                    checks.  When using this status, consider explaining the
                    situation in the slot's notes.
                </p>
            </li>
            <li>
                <span class="fw-bold text-success">Go mode</span>: This slot's
                goal can be completed.
            </li>
        </ul>
        <p>
            Completion status communicates how complete the slot is.
        </p>
        <table class="table text-center">
            <thead>
                <tr>
                    <th>Status</th>
                    <th>All checks obtained</th>
                    <th>Goal complete</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td class="fw-bold text-light">Incomplete</td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
                <tr>
                    <td class="fw-bold text-info">All checks</td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
                <tr>
                    <td class="fw-bold text-info">Goal</td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                </tr>
                <tr>
                    <td class="fw-bold text-success">Done</td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                </tr>
                <tr>
                    <td class="fw-bold text-muted align-middle">Released</td>
                    <td colspan="2">
                        The owner of this slot has abandoned it and released the
                        items it held because the owner does not think they can
                        complete it. This could be due to an unexpected personal
                        situation, the slot's settings proved too challenging,
                        or some other reason. Before you release a slot, consult
                        with the organizer of your multiworld. It's likely
                        someone else can take it over instead.
                    </td>
                </tr>
            </tbody>
        </table>
        <p>
            If a slot is not currently marked "released," then its status will
            be automatically changed to "all checks," "goal," or "done" when the
            Archipelago tracker indicates that the slot meets the relevant
            criteria.
        </p>
        <p>
            You can manually select a more complete status than indicated by the
            Archipelago tracker, but not a less complete status. This is useful
            for games where not all checks are possible (in slots with
            accessibilty set to "minimal" during generation, for example), which
            can be set to "done" when there is no more possible progression even
            though not all checks have been obtained. You can also always select
            "released" -- this status supersedes all others.
        </p>
        <p>
            In particular, note that if the automatic status would be "all
            checks" or "goal" and you try to manually set the other one, this is
            interpreted as "done" (all checks + goal = done).
        </p>
        <h2>Hints and Notes</h2>
        <p>
            The tracker captures information about what hints have been issued
            and whether the hinted item has been found. This information is
            organized into the "hints and notes" pane, which can be viewed by
            clicking the button in the "notes" column for a particular slot.
        </p>
        <p>
            On the left, you will see a list of hints. By default, this will
            show <i>received hints</i>, which are hints for which the slot is
            the "finder" -- these are items that exist in the slot's game that
            need to be found for another slot. You can click the "sent hints"
            button to instead show hints sent <i>from</i> the slot. (Note that
            this is a global toggle, so it will cause every hints pane to show
            sent hints. However, it does not affect the counter in the hint
            column, which always shows received hints.)
        </p>
        <p>
            Each hint can be classified, which communicates how important the
            item is to the receiving slot:
        </p>
        <ul>
            <li>
                <span class="fw-bold text-danger">Critical</span>: The item is
                required. This could be because it's necessary for the receiving
                slot to be able to goal, or because the receiving slot needs it
                to obtain a critical item for another slot.
            </li>
            <li>
                <span class="fw-bold text-warning">Helpful</span>: The item is
                not required but would be helpful in some capacity, so the item
                may be worth obtaining if it is not too difficult or out of the
                way.
            </li>
            <li>
                <span class="fw-bold text-secondary">Trash</span>: The item is
                worthless to the receiving slot and no effort should be spent on
                obtaining it.
            </li>
        </ul>
        <p>
            There is some potential ambiguity where it's critical that a slot
            needs one out of a set of items, but not all of them.  This
            situation is best handled by communicating with the other players on
            Discord or by using the notes field.  For example, if you need one
            of two items to goal but you don't need both, you could ask the
            sending slots' players how difficult it would be to get each item
            and choose which one to mark critical based on which is easier to
            obtain.  Another approach would be to mark both critical and then
            downgrade one to helpful or trash when the other one is obtained.
        </p>
        <p>
            By default, hints are shown only if all of the following are true.
            Additionally, only hints matching these criteria are considered for
            the count shown in the hints column.
        </p>
        <ul>
            <li>The item has not been found yet.</li>
            <li>
                The slot that receives the item does not have the <span
                class="fw-bold text-success">done</span> completion status.
            </li>
            <li>The hint is not classified as trash.</li>
        </ul>
        <p>
            You can toggle the "include found and useless hints" option to
            display all hints.  This will not change the count shown in the hint
            column.
        </p>
        <p>
            On the right side of the pane, you will see a notes field. This is
            intended for you to make notes for yourself about your slot. It may
            also be used to make notes for others, though be aware that not
            every player is going to read every other players' notes, especially
            given that there is no indication when a note has been updated.
        </p>
        <p>
            Notes are saved when you click off of the text field. If you make a
            mistake, you can press the escape (ESC) key on your keyboard to
            revert any changes.
        </p>
        <h2><a name="pinging"/>Pinging</h2>
        <p>
            There are mulitple scenarios where the owner of a slot may want to
            be notified on Discord with a ping. The table below describes these
            scenarios and whether it is appropriate to ping someone based on the
            ping preference set on their slot.
        </p>
        <table class="table text-center">
            <thead>
                <tr>
                    <th>Scenario</th>
                    <th>Liberally</th>
                    <th>Sparingly</th>
                    <th>Hints</th>
                    <th>See notes</th>
                    <th>Never</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>You hint an item found by the slot.</td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="notes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
                <tr>
                    <td>You find an item that was hinted by the slot.</td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="notes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
                <tr>
                    <td>You get a progression item you are 100% sure is critical.</td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="notes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
                <tr>
                    <td>You get a progression item you are not 100% sure is critical.</td>
                    <td><yes-no value="yes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="notes"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
                <tr>
                    <td>You are getting impatient not receiving a hinted item.</td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                    <td><yes-no value="no"></yes-no></td>
                </tr>
            </tbody>
        </table>
        <div class="row text-center">
            <div class="col-4">
                <yes-no value="yes"></yes-no> You can ping
            </div>
            <div class="col-4">
                <yes-no value="notes"></yes-no> Check the slot's notes to see if you can ping
            </div>
            <div class="col-4">
                <yes-no value="no"></yes-no> Do not ping
            </div>
        </div>
        <h2>Reports</h2>
        <p>
            At the bottom of the tracker, there are a few reports to help the
            organizer and players see how the multiworld is progressing at a
            glance.
        </p>
        <p>
            <i>
                Note that all information in the report excludes slots marked
                "released."
            </i>
            These slots are considered to be removed from the multiworld, so
            their checks do not count for or against progression.
        </p>
        <p>
            Unique players, unique games, and total checks are all
            self-explanatory.
        </p>
        <p>
            The status summary shows two bars, which helps visualise the status
            information of all of the slots at once.
        </p>
        <ul>
            <li>
                <p>
                    <b>Progression</b> shows an aggregate report about checks and is
                    intended to visualize the entire progress of the whole
                    multiworld. The green bar represents completed checks, which
                    means it contains all checks for "done" and "all checks" slots
                    <i>as well as completed checks from slots that have not been
                        completed themselves.</i> This allows you to see at a glance
                    (roughly) what percentage of <i>checks</i> are in unblocked or
                    BK slots.
                </p>
                <p>
                    The colors represent the corresponding progression status
                    for remaining checks, except for green which represents
                    completed checks.
                </p>
            </li>
            <li>
                <p>
                    <b>Completion</b> is far simpler and shows how many slots
                    are in which status. While this is interesting data, note
                    that interpreting this in terms of "how much is left to do
                    in the multiworld" can cause slots with very few checks
                    (such as Clique) to skew your interpretation. (This is why
                    the "progression" bar exists.)
                </p>
                <p>
                    The colors represent the corresponding completion status of
                    each slot, except for red. The red section represents
                    incomplete and BK slots, which are <i>not</i> represented in
                    the white (incomplete) section.
                </p>
            </li>
        </ul>
        <p>
            Below these bars, there are two tables, which simply break down the
            slots two ways: one by player, and one by game. The "slots" column
            of these tables contains a bar showing the completion status of each
            slot, the same way that the global "completion" bar does.
        </p>
    </div>
</template>
