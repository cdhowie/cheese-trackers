<script setup>
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
            The very first thing you should do is head over to the
            <router-link to="/settings">settings</router-link> and put in your
            Discord username. You must do this before you can claim any slots.
        </p>
        <h2>Tracker Title</h2>
        <p>
            Every tracker has a title at the top. By default, this will say
            <span class="text-muted fst-italic">untitled tracker</span>. You can
            click this text to create a title, which will be displayed at the
            top of the sheet as well as in the browser tab and/or title bar.
            This can help players who are in a large number of multiworlds
            better manage their tracker tabs.
        </p>
        <h2>Tracker Columns</h2>
        <p>
            When you open a tracker, you will be presented with a large table.
            Each row in this table corresponds to one of the slots in the
            multiworld. Here is a list of the columns of this table and what
            they represent:
        </p>
        <ul>
            <li>
                <b>Name</b>: This is the name of the slot on Archipelago. If you
                alias your slot, it will be reflected here.
            </li>
            <li>
                <b>Ping</b>: Whether the owner of this slot wants to be pinged
                on Discord regarding this slot. See the "pinging" section below
                for more information about what this means and when it is
                appropriate to ping someone.
            </li>
            <li>
                <p>
                    <b>Owner</b>: The Discord username of the player who has
                    claimed this slot. If this is not your slot, a "claim"
                    button will appear, which can be used to claim the slot for
                    yourself. If this is your slot, a "release" button will
                    appear instead, allowing you to release the slot for someone
                    else to claim. It goes without saying that you should not
                    claim a slot owned by another player without consulting them
                    or the organizer of the multiworld first. (An additional
                    popup confirmation will appear when you attempt to claim
                    someone else's slot to help prevent accidents.)
                </p>
                <p>
                    Clicking the title of this column will allow you to filter
                    slots by a specific owner. This is a good way to show only
                    your own slots, particularly if you own a large number of
                    slots.
                </p>
            </li>
            <li>
                <b>Game</b>: The game being played in this slot.
            </li>
            <li>
                <p>
                    <b>Status</b>: The status of the game in this slot. See the
                    "status" section below for more information about what the
                    various statuses mean.
                </p>
                <p>
                    Clicking the title of this column will allow you to filter
                    slots by any combination of statuses.
                </p>
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
                <p>
                    Clicking the title of this column will allow you to filter
                    slots to those whose last activity was more than a certain
                    number of days in the past.
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
                <p>
                    Clicking the title of this column will expand or collapse
                    the hints and notes panes for all slots.
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
        <h2>Status</h2>
        <p>
            The status column is used to communicate what is happening with a
            particular slot. The following statuses are available:
        </p>
        <ul>
            <li>
                <span class="fw-bold text-light">Unblocked</span>: This slot has
                available progression. The owner may or may not be playing it at
                this moment, but they are aware that progression is available.
            </li>
            <li>
                <span class="fw-bold text-danger">BK</span>: This slot has no
                available progression, and is waiting on items from other slots.
            </li>
            <li>
                <span class="fw-bold text-warning">All checks</span>: This slot
                has sent all of its checks but its goal is not yet complete.
            </li>
            <li>
                <span class="fw-bold text-success">Done</span>: This slot
                has sent all of its checks and its goal is complete.
            </li>
            <li>
                <span class="fw-bold text-info">Open</span>: This slot is open
                for a player to claim it. This may be because the prior owner
                has abandoned it, or the organizer of the multiworld
                intentionally included some unclaimed slots so that new players
                can join the multiworld after it starts. Note that the organizer
                of a multiworld may have restrictions around who can claim open
                slots, so it may be prudent to consult them before claiming an
                open slot on the tracker.
            </li>
            <li>
                <span class="fw-bold text-muted">Released</span>: The owner of
                this slot has abandoned it and released the items it held
                because the owner does not think they can complete it. This
                could be due to an unexpected personal situation, the slot's
                settings proved too challenging, or some other reason. Before
                you release a slot, consult with the organizer of your
                multiworld. It's likely someone else can take it over instead.
            </li>
            <li>
                <span class="fw-bold text-muted">Glitched</span>: The owner of
                this slot has abandoned it and released the items it held
                because a generation issue is preventing progress or completion
                of the slot.
            </li>
        </ul>
        <p>
            There are a few status changes that are automatically made when
            certain conditions are met:
        </p>
        <ul>
            <li>
                If a slot has completed all of its checks and is not currently
                marked "released" or "glitched," then its status will be changed
                to "done" if the slot's goal has been completed, or "all checks"
                if the slot's goal has not been completed.
            </li>
            <li>
                If a slot is marked "BK" and its number of completed checks
                increases, its status will be changed to "Unblocked."
            </li>
        </ul>
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
            Note that only hints for items not yet found are displayed. A hint
            will be removed once its item has been found, as there is no longer
            any use for the hint.
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
        <h2>Pinging</h2>
        <p>
            When an important item is sent to a slot, the owner of that slot may
            want to be informed on Discord. Check the "ping" column for a slot
            first to see if the owner wants to receive pings.
        </p>
        <p>
            <span class="text-warning">
                If the column says "no" or the button is missing then you should
                never ping the slot's owner about items you've sent to them.
            </span>
            (The button is hidden for completed, released, or glitched slots.
            There is no reason to ping in those cases.)
        </p>
        <p>
            If the column says "yes," then it <i>might</i> be appropriate to
            ping them. You should follow these rules for pinging (adapted from
            Dragorrod's rules to fit the terminology of this tracker):
        </p>
        <ul>
            <li>
                You hint something on someone else,
                <span class="text-success">you ping</span>.
            </li>
            <li>
                You are getting impatient not receiving a hinted item,
                <span class="text-danger">you do not ping</span>.
            </li>
            <li>
                You get a progression item you are not 100% sure is critical,
                <span class="text-danger">you do not ping</span>.
            </li>
            <li>
                You get a progression item you are 100% sure is critical,
                <span class="text-success">you can ping</span>, but you do not
                have to.
            </li>
        </ul>
        <h2>Reports</h2>
        <p>
            At the bottom of the tracker, there are a few reports to help the
            organizer and players see how the multiworld is progressing at a
            glance.
        </p>
        <p>
            <i>
                Note that all information in the report excludes slots marked
                "released" or "glitched."
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
            information of all of the slots at once. The colors used reflect
            statuses, so white means "unblocked," red means "BK," and so on.
            Consult the "status" section to see the colors for each status.
        </p>
        <ul>
            <li>
                <b>Progression</b> shows an aggregate report about checks and is
                intended to visualize the entire progress of the whole
                multiworld. The green bar represents completed checks, which
                means it contains all checks for "done" and "all checks" slots
                <i>as well as completed checks from slots that have not been
                    completed themselves.</i> This allows you to see at a glance
                (roughly) what percentage of <i>checks</i> are in unblocked or
                BK slots.
            </li>
            <li>
                <b>By game</b> is far simpler and shows how many slots are in
                which status. While this is interesting data, note that
                interpreting this in terms of "how much is left to do in the
                multiworld" can cause games with very few checks (such as
                Clique) to skew your interpretation. (This is why the
                "progression" bar exists.)
            </li>
        </ul>
        <p>
            Below these bars, there are two tables, which simply break down the
            slots two ways: one by player, and one by game.
        </p>
    </div>
</template>
