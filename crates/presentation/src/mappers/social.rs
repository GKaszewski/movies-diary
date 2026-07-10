use api_types::RemoteActorDto;

pub fn remote_actor_to_dto(a: activitypub::RemoteActor) -> RemoteActorDto {
    RemoteActorDto {
        handle: a.handle,
        display_name: a.display_name,
        url: a.url,
    }
}
