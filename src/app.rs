use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::components::editor::*;
use crate::components::home::*;
use crate::components::nav::*;
use crate::components::study::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/tusk.css"/>
        <Link rel="icon" href="/favicon.ico"/>
        <Title text="Tusk 🐘"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Router>
            <Nav/>
            <main class="max-w-5xl mx-auto px-4 py-8">
                <Routes>
                    <Route path="/" view=HomePage/>
                    <Route path="/study" view=StudyPage/>
                    <Route path="/editor" view=EditorPage/>
                    <Route path="/editor/:id" view=EditorPage/>
                </Routes>
            </main>
        </Router>
    }
}
